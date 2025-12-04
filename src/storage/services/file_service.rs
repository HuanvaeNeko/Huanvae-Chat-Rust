use chrono::Utc;
use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use tracing::info;

use crate::common::{generate_conversation_uuid, AppError};
use crate::config::storage_config;
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::{DeduplicationService, FileValidator, UuidMappingService};

/// 统一文件上传服务
pub struct FileService {
    db: PgPool,
    s3_client: Arc<S3Client>,
    dedup_service: Arc<DeduplicationService>,
    uuid_mapping_service: Arc<UuidMappingService>,
    api_base_url: String,
}

impl FileService {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>, api_base_url: String) -> Self {
        let dedup_service = Arc::new(DeduplicationService::new(db.clone(), s3_client.clone()));
        let uuid_mapping_service = Arc::new(UuidMappingService::new(db.clone()));
        Self {
            db,
            s3_client,
            dedup_service,
            uuid_mapping_service,
            api_base_url,
        }
    }

    /// 请求上传（统一入口）
    pub async fn request_upload(
        &self,
        user_id: &str,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, AppError> {
        // 1. 验证哈希格式（如果提供了哈希）
        let file_hash = request.file_hash.as_deref().unwrap_or("no_hash");
        if request.file_hash.is_some() {
            FileValidator::validate_hash(file_hash)?;
        }

        // 2. 判断文件类型和预览支持
        let is_friend_message = matches!(
            request.storage_location,
            StorageLocation::FriendMessages
        );
        let (_file_type, preview_support) = FileValidator::determine_file_type_and_preview(
            &request.content_type,
            request.file_size,
            is_friend_message,
        )?;

        // 3. 验证文件类型和大小
        FileValidator::validate_file_type(&request.file_type, &request.content_type)?;
        FileValidator::validate_file_size(&request.file_type, request.file_size)?;

        // 4. 生成唯一file_key（按照MinIO/data.md规范）
        let file_key = self.generate_file_key(
            &request.storage_location,
            &request.file_type,
            user_id,
            request.related_id.as_deref(),
            &request.filename,
            file_hash,
        );

        // 5. 秒传检查（仅当提供了哈希且force_upload=false时）
        if !request.force_upload.unwrap_or(false) && request.file_hash.is_some() {
            if let Some(existing) = self.dedup_service
                .check_and_create_uuid_reference(
                    file_hash,
                    user_id,
                    &request.file_type,
                    &request.storage_location,
                    request.related_id.as_deref(),
                    &file_key,
                    request.file_size as i64,
                    &request.content_type,
                    &preview_support,
                )
                .await?
            {
                info!("秒传成功(UUID映射): 用户 {} 复用文件 {}", user_id, existing.file_key);
                return Ok(FileUploadResponse {
                    mode: UploadMode::PresignedPut,  // 秒传使用预签名模式标识
                    preview_support,
                    upload_token: None,
                    upload_url: None,
                    expires_in: None,
                    presigned_url: None,
                    multipart_upload_id: None,
                    file_key: existing.file_key.clone(),
                    max_file_size: 0,
                    instant_upload: true,
                    existing_file_url: Some(existing.file_url),
                    message_uuid: None,
                    message_send_time: None,
                });
            }
        }

        // 6. 判断上传模式
        let upload_mode = FileValidator::determine_upload_mode(request.file_size);

        // 7. 根据模式生成上传凭证（全部预签名直传MinIO）
        let response = match upload_mode {
            UploadMode::PresignedPut => {
                self.generate_presigned_put_upload(
                    user_id,
                    &file_key,
                    &request,
                    preview_support,
                ).await?
            }
            UploadMode::PresignedMultipart => {
                self.generate_presigned_multipart_upload(
                    user_id,
                    &file_key,
                    &request,
                    preview_support,
                ).await?
            }
        };

        Ok(response)
    }

    /// 生成预签名PUT上传URL（< 5GB，直传MinIO）
    async fn generate_presigned_put_upload(
        &self,
        user_id: &str,
        file_key: &str,
        request: &FileUploadRequest,
        preview_support: PreviewSupport,
    ) -> Result<FileUploadResponse, AppError> {
        let file_hash = request.file_hash.as_deref().unwrap_or("no_hash");
        let bucket = request.storage_location.to_bucket_name();
        
        // 计算有效期
        let expires_in = FileValidator::calculate_expires_in(request.file_size, request.estimated_upload_time);
        
        // 生成预签名PUT URL（直传MinIO）
        let presigned_url = self.s3_client
            .generate_presigned_upload_url(bucket, file_key, &request.content_type, expires_in)
            .await?;

        // 创建数据库记录（状态为 pending_confirm）
        self.create_pending_confirm_record(
            file_key,
            user_id,
            &request.file_type,
            &request.storage_location,
            request.related_id.as_deref(),
            request.file_size,
            &request.content_type,
            file_hash,
            expires_in,
            &preview_support,
        ).await?;

        Ok(FileUploadResponse {
            mode: UploadMode::PresignedPut,
            preview_support,
            upload_token: None,
            upload_url: None,
            expires_in: Some(expires_in),
            presigned_url: Some(presigned_url),
            multipart_upload_id: None,
            file_key: file_key.to_string(),
            max_file_size: FileValidator::get_max_file_size(&request.file_type),
            instant_upload: false,
            existing_file_url: None,
            message_uuid: None,
            message_send_time: None,
        })
    }

    /// 生成预签名分片上传（>= 5GB）
    async fn generate_presigned_multipart_upload(
        &self,
        user_id: &str,
        file_key: &str,
        request: &FileUploadRequest,
        preview_support: PreviewSupport,
    ) -> Result<FileUploadResponse, AppError> {
        let file_hash = request.file_hash.as_deref().unwrap_or("no_hash");
        
        let expires_in = request.estimated_upload_time
            .ok_or_else(|| AppError::BadRequest("超大文件必须指定预计上传时间".to_string()))?;
        
        let bucket = request.storage_location.to_bucket_name();
        
        // 初始化分片上传
        let upload_id = self.s3_client
            .initiate_multipart_upload(bucket, file_key, &request.content_type)
            .await?;

        // 创建数据库记录（状态为 pending_confirm）
        self.create_pending_confirm_record(
            file_key,
            user_id,
            &request.file_type,
            &request.storage_location,
            request.related_id.as_deref(),
            request.file_size,
            &request.content_type,
            file_hash,
            expires_in,
            &preview_support,
        ).await?;

        // 保存upload_id
        sqlx::query(
            r#"UPDATE "file-records" SET "multipart-upload-id" = $1 WHERE "file-key" = $2"#
        )
        .bind(&upload_id)
        .bind(file_key)
        .execute(&self.db)
        .await?;

        Ok(FileUploadResponse {
            mode: UploadMode::PresignedMultipart,
            preview_support,
            upload_token: None,
            upload_url: None,
            expires_in: Some(expires_in),
            presigned_url: None,
            multipart_upload_id: Some(upload_id),
            file_key: file_key.to_string(),
            max_file_size: FileValidator::get_max_file_size(&request.file_type),
            instant_upload: false,
            existing_file_url: None,
            message_uuid: None,
            message_send_time: None,
        })
    }

    /// 生成文件key（严格按照MinIO/data.md规范）
    fn generate_file_key(
        &self,
        storage_location: &StorageLocation,
        file_type: &FileType,
        user_id: &str,
        related_id: Option<&str>,
        filename: &str,
        file_hash: &str,
    ) -> String {
        let timestamp = Utc::now().timestamp();
        let hash_prefix = &file_hash[0..8];
        let extension = FileValidator::get_extension(filename);
        let sanitized_name = FileValidator::sanitize_filename(filename);

        match storage_location {
            StorageLocation::Avatars => {
                // avatars/{user_id}.ext
                format!("{}.{}", user_id, extension)
            }
            StorageLocation::UserFiles => {
                // user-file/{user_id}/{type}/{timestamp}_{hash}_{filename}.ext
                let type_dir = match file_type {
                    FileType::UserImage | FileType::UserImageFile => "images",
                    FileType::UserVideo | FileType::UserVideoFile => "videos",
                    FileType::UserDocument => "files",
                    _ => "files",
                };
                format!("{}/{}/{}_{}_{}",
                    user_id, type_dir, timestamp, hash_prefix, sanitized_name)
            }
            StorageLocation::FriendMessages => {
                // friends-file/{conversation-uuid}/{type}/{timestamp}_{hash}_{filename}.ext
                let friend_id = related_id.unwrap_or("unknown");
                let conversation_uuid = generate_conversation_uuid(user_id, friend_id);
                let type_dir = match file_type {
                    FileType::FriendImage | FileType::FriendImageFile => "images",
                    FileType::FriendVideo | FileType::FriendVideoFile => "videos",
                    FileType::FriendDocument => "files",
                    _ => "files",
                };
                format!("{}/{}/{}_{}_{}",
                    conversation_uuid, type_dir, timestamp, hash_prefix, sanitized_name)
            }
            StorageLocation::GroupFiles => {
                // group-file/group-{group_id}/{type}/{timestamp}_{hash}_{filename}.ext
                let group_id = related_id.unwrap_or("unknown");
                let type_dir = match file_type {
                    FileType::GroupImage => "images",
                    FileType::GroupVideo => "videos",
                    FileType::GroupDocument => "files",
                    _ => "files",
                };
                format!("group-{}/{}/{}_{}_{}",
                    group_id, type_dir, timestamp, hash_prefix, sanitized_name)
            }
        }
    }

    /// 创建待确认的文件记录（预签名上传专用，状态为 pending_confirm）
    async fn create_pending_confirm_record(
        &self,
        file_key: &str,
        owner_id: &str,
        file_type: &FileType,
        storage_location: &StorageLocation,
        related_id: Option<&str>,
        file_size: u64,
        content_type: &str,
        file_hash: &str,
        expires_in: u32,
        preview_support: &PreviewSupport,
    ) -> Result<(), AppError> {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        
        sqlx::query(
            r#"INSERT INTO "file-records" 
            ("file-key", "owner-id", "file-type", "storage-location", "related-id",
             "file-size", "content-type", "file-hash", "status",
             "created-at", "expires-at", "preview-support")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending_confirm', NOW(), $9, $10)
            ON CONFLICT ("file-key") DO NOTHING"#
        )
        .bind(file_key)
        .bind(owner_id)
        .bind(file_type.to_string())
        .bind(storage_location.to_string())
        .bind(related_id)
        .bind(file_size as i64)
        .bind(content_type)
        .bind(file_hash)
        .bind(expires_at)
        .bind(preview_support.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }
    
    /// 验证并完成预签名上传（确认文件已上传到MinIO）
    pub async fn verify_and_complete_presigned_upload(
        &self,
        file_key: &str,
        user_id: &str,
    ) -> Result<FileRecord, AppError> {
        // 1. 查询待确认的文件记录
        let record = sqlx::query_as::<_, FileRecord>(
            r#"SELECT * FROM "file-records"
            WHERE "file-key" = $1
              AND "owner-id" = $2
              AND "status" = 'pending_confirm'
              AND "expires-at" > NOW()"#
        )
        .bind(file_key)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("文件记录不存在或已过期".to_string()))?;

        // 2. 验证MinIO中文件确实存在
        let storage_loc: StorageLocation = record.storage_location.parse()
            .map_err(|e: String| AppError::BadRequest(e))?;
        let bucket = self.get_bucket_name(&storage_loc);
        
        if !self.s3_client.file_exists(bucket, file_key).await? {
            return Err(AppError::BadRequest("文件未上传到存储服务".to_string()));
        }

        Ok(record)
    }

    /// 完成预签名上传（创建UUID映射、权限授予）
    pub async fn complete_presigned_upload(
        &self,
        file_key: &str,
        owner_id: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &str,
        storage_location: &str,
        related_id: Option<&str>,
        file_hash: &str,
    ) -> Result<String, AppError> {
        // 创建UUID映射
        let file_uuid = self.uuid_mapping_service
            .create_mapping(file_key, file_hash, file_size, content_type, preview_support, owner_id)
            .await?;
        
        // 授予上传者权限
        self.uuid_mapping_service
            .grant_permission(&file_uuid, owner_id, "owner", "upload")
            .await?;
        
        // 好友文件：同时授权好友访问
        if storage_location == "friend_messages" {
            if let Some(friend_id) = related_id {
                info!("好友文件上传完成，授权好友 {} 访问", friend_id);
                self.uuid_mapping_service
                    .grant_permission(&file_uuid, friend_id, "read", "friend_share")
                    .await?;
            }
        }

        // 群文件：授予所有活跃群成员读取权限
        if storage_location == "group_files" {
            if let Some(group_id_str) = related_id {
                info!("群文件上传完成，授权群 {} 所有成员访问", group_id_str);
                let members: Vec<(String,)> = sqlx::query_as(
                    r#"SELECT "user-id" FROM "group-members" 
                       WHERE "group-id" = $1::uuid AND "status" = 'active'"#
                )
                .bind(&group_id_str)
                .fetch_all(&self.db)
                .await?;

                for (member_id,) in members {
                    if member_id != owner_id {
                        self.uuid_mapping_service
                            .grant_permission(&file_uuid, &member_id, "read", "group_share")
                            .await?;
                    }
                }
            }
        }
        
        // 生成UUID访问URL
        let uuid_file_url = format!("{}/api/storage/file/{}", self.api_base_url, file_uuid);
        
        // 更新file-records
        sqlx::query(
            r#"UPDATE "file-records" 
            SET "status" = 'completed',
                "file-url" = $2,
                "file-uuid" = $3,
                "completed-at" = NOW()
            WHERE "file-key" = $1
              AND "status" = 'pending_confirm'"#
        )
        .bind(file_key)
        .bind(&uuid_file_url)
        .bind(&file_uuid)
        .execute(&self.db)
        .await?;

        Ok(uuid_file_url)
    }

    /// 验证并获取上传Token信息
    pub async fn verify_and_get_upload_token(
        &self,
        token: &str,
    ) -> Result<FileRecord, AppError> {
        let record = sqlx::query_as::<_, FileRecord>(
            r#"SELECT * FROM "file-records"
            WHERE "upload-token" = $1
              AND "status" = 'pending'
              AND "expires-at" > NOW()"#
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("Token无效或已过期".to_string()))?;

        Ok(record)
    }

    /// 完成上传（消费Token）并创建UUID映射
    pub async fn complete_upload_with_token(
        &self,
        token: &str,
        actual_hash: &str,
        file_key: &str,
        owner_id: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &str,
    ) -> Result<String, AppError> {
        // 查询文件记录获取 storage_location 和 related_id
        let record: Option<(String, Option<String>)> = sqlx::query_as(
            r#"SELECT "storage-location", "related-id" FROM "file-records" 
            WHERE "upload-token" = $1 AND "status" = 'pending'"#
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?;

        let (storage_location, related_id) = record
            .ok_or_else(|| AppError::BadRequest("Token无效或已过期".to_string()))?;

        // 创建UUID映射
        let file_uuid = self.uuid_mapping_service
            .create_mapping(file_key, actual_hash, file_size, content_type, preview_support, owner_id)
            .await?;
        
        // 授予上传者权限
        self.uuid_mapping_service
            .grant_permission(&file_uuid, owner_id, "owner", "upload")
            .await?;
        
        // 好友文件：同时授权好友访问
        if storage_location == "friend_messages" {
            if let Some(ref friend_id) = related_id {
                info!("好友文件上传完成，授权好友 {} 访问", friend_id);
                self.uuid_mapping_service
                    .grant_permission(&file_uuid, friend_id, "read", "friend_share")
                    .await?;
            }
        }

        // 群文件：授予所有活跃群成员读取权限
        if storage_location == "group_files" {
            if let Some(ref group_id_str) = related_id {
                info!("群文件上传完成，授权群 {} 所有成员访问", group_id_str);
                // 查询群内所有活跃成员
                let members: Vec<(String,)> = sqlx::query_as(
                    r#"SELECT "user-id" FROM "group-members" 
                       WHERE "group-id" = $1::uuid AND "status" = 'active'"#
                )
                .bind(&group_id_str)
                .fetch_all(&self.db)
                .await?;

                for (member_id,) in members {
                    if member_id != owner_id {
                        self.uuid_mapping_service
                            .grant_permission(&file_uuid, &member_id, "read", "group_share")
                            .await?;
                    }
                }
            }
        }
        
        // 生成UUID访问URL
        let uuid_file_url = format!("{}/api/storage/file/{}", self.api_base_url, file_uuid);
        
        // 更新file-records
        let result = sqlx::query(
            r#"UPDATE "file-records" 
            SET "status" = 'completed',
                "upload-token" = NULL,
                "file-url" = $3,
                "file-uuid" = $4,
                "completed-at" = NOW()
            WHERE "upload-token" = $1
              AND "file-hash" = $2
              AND "status" = 'pending'"#
        )
        .bind(token)
        .bind(actual_hash)
        .bind(&uuid_file_url)
        .bind(&file_uuid)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("Token无效或哈希不匹配".to_string()));
        }

        Ok(uuid_file_url)
    }

    /// 获取bucket名称
    pub fn get_bucket_name(&self, storage_location: &StorageLocation) -> &str {
        storage_location.to_bucket_name()
    }

    /// 获取UUID映射信息
    pub async fn get_uuid_mapping(&self, file_uuid: &str) -> Result<Option<crate::storage::services::uuid_mapping::UuidMappingInfo>, AppError> {
        self.uuid_mapping_service.get_by_uuid(file_uuid).await
    }

    /// 生成文件URL
    pub fn generate_file_url(&self, storage_location: &StorageLocation, file_key: &str) -> String {
        let bucket = storage_location.to_bucket_name();
        format!("{}/{}/{}", self.s3_client.config().public_url, bucket, file_key)
    }

    /// 生成分片上传URL
    pub async fn generate_multipart_part_url(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        user_id: &str,
    ) -> Result<MultipartPartResponse, AppError> {
        // 验证upload_id属于该用户
        let record = sqlx::query_as::<_, FileRecord>(
            r#"SELECT * FROM "file-records"
            WHERE "file-key" = $1
              AND "multipart-upload-id" = $2
              AND "owner-id" = $3
              AND "status" = 'pending'"#
        )
        .bind(file_key)
        .bind(upload_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("无效的upload_id".to_string()))?;

        let storage_loc: StorageLocation = record.storage_location.parse()
            .map_err(|e: String| AppError::BadRequest(e))?;
        let bucket = self.get_bucket_name(&storage_loc);
        
        let multipart_ttl = storage_config().multipart_url_ttl;
        let part_url = self.s3_client
            .generate_presigned_upload_part_url(bucket, file_key, upload_id, part_number, multipart_ttl)
            .await?;

        Ok(MultipartPartResponse {
            part_url,
            part_number,
            expires_in: multipart_ttl,
        })
    }

    /// 生成文件预签名下载URL（通过UUID访问）
    pub async fn generate_presigned_url(
        &self,
        user_id: &str,
        file_uuid: &str,
        expires_in: u32,
    ) -> Result<PresignedUrlResponse, AppError> {
        // 1. 查询UUID映射表获取物理文件信息
        let mapping: UuidMappingRecord = sqlx::query_as(
            r#"
            SELECT "uuid", "physical-file-key", "file-hash", "file-size", "content-type",
                   "preview-support", "first-uploader-id", "created-at"
            FROM "file-uuid-mapping"
            WHERE "uuid" = $1
            "#
        )
        .bind(file_uuid)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("文件".to_string()))?;

        // 2. 验证用户权限
        let _permission: PermissionRecord = sqlx::query_as(
            r#"
            SELECT "id", "access-type", "granted-at", "revoked-at"
            FROM "file-access-permissions"
            WHERE "file-uuid" = $1 AND "user-id" = $2 AND "revoked-at" IS NULL
            "#
        )
        .bind(file_uuid)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::Forbidden)?;

        // 3. 根据physical_file_key前缀判断bucket
        // physical_file_key格式:
        //   - 个人文件: {user_id}/{type}/{timestamp}_{hash}_{filename}
        //   - 好友文件: conv-{user1}-{user2}/{type}/{timestamp}_{hash}_{filename}
        //   - 群文件: group-{group_uuid}/{type}/{timestamp}_{hash}_{filename}
        let physical_file_key = &mapping.physical_file_key;
        let bucket = if physical_file_key.starts_with("conv-") {
            // 好友消息文件
            "friends-file"
        } else if physical_file_key.starts_with("group-") {
            // 群文件
            "group-file"
        } else {
            // 个人文件
            "user-file"
        };

        // 4. 生成预签名下载URL
        let presigned_url = self
            .s3_client
            .generate_presigned_download_url(bucket, physical_file_key, expires_in)
            .await?;

        // 5. 计算过期时间
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);
        let expires_at_str = expires_at.to_rfc3339();

        // 6. 生成警告信息（如果有效期超过1天）
        let warning = if expires_in > 86400 {
            Some(format!(
                "此链接将在{}小时后过期",
                expires_in / 3600
            ))
        } else {
            None
        };

        Ok(PresignedUrlResponse {
            presigned_url,
            expires_at: expires_at_str,
            file_uuid: file_uuid.to_string(),
            file_size: mapping.file_size,
            content_type: mapping.content_type,
            warning,
        })
    }

    /// 查询用户文件列表（支持分页、过滤、排序）
    /// 只返回用户个人文件（storage_location = 'user_files'），不包含好友和群聊文件
    pub async fn list_user_files(
        &self,
        user_id: &str,
        page: i32,
        limit: i32,
        sort_by: String,
        sort_order: String,
    ) -> Result<FileListResponse, AppError> {
        // 1. 参数验证
        let page = page.max(1);
        let limit = limit.clamp(1, 100);
        let offset = (page - 1) * limit;
        
        // 2. 确定排序字段（用于子查询外部排序，字段来自子查询结果）
        let sort_column = match sort_by.as_str() {
            "file_size" => r#""file-size""#,
            _ => r#""created-at""#,
        };
        let sort_dir = if sort_order == "asc" { "ASC" } else { "DESC" };
        
        // 3. 查询总数（只统计个人文件）
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT m."uuid") as count
            FROM "file-uuid-mapping" m
            INNER JOIN "file-access-permissions" p ON m."uuid" = p."file-uuid"
            INNER JOIN "file-records" r ON r."file-uuid" = m."uuid"
            WHERE p."user-id" = $1 
              AND p."revoked-at" IS NULL
              AND r."owner-id" = $1
              AND r."storage-location" = 'user_files'
            "#
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;
        
        // 4. 构建查询SQL（只查询个人文件，使用子查询去重后排序）
        let query_sql = format!(
            r#"
            SELECT * FROM (
                SELECT DISTINCT ON (m."uuid")
                    m."uuid", m."physical-file-key", m."file-size", 
                    m."content-type", m."preview-support", m."created-at"
                FROM "file-uuid-mapping" m
                INNER JOIN "file-access-permissions" p ON m."uuid" = p."file-uuid"
                INNER JOIN "file-records" r ON r."file-uuid" = m."uuid"
                WHERE p."user-id" = $1 
                  AND p."revoked-at" IS NULL
                  AND r."owner-id" = $1
                  AND r."storage-location" = 'user_files'
                ORDER BY m."uuid"
            ) AS unique_files
            ORDER BY {} {}
            LIMIT $2 OFFSET $3
            "#,
            sort_column, sort_dir
        );
        
        // 5. 执行查询
        let rows = sqlx::query(&query_sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?;
        
        // 6. 转换为响应格式
        let files: Vec<FileItem> = rows
            .into_iter()
            .map(|row| {
                let uuid: String = row.try_get("uuid").unwrap_or_default();
                let physical_key: String = row.try_get("physical-file-key").unwrap_or_default();
                let filename = Self::extract_filename_from_key(&physical_key);
                
                FileItem {
                    file_uuid: uuid.clone(),
                    filename,
                    file_size: row.try_get("file-size").unwrap_or(0),
                    content_type: row.try_get("content-type").unwrap_or_default(),
                    preview_support: row.try_get("preview-support").unwrap_or_default(),
                    created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created-at")
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    file_url: format!("{}/api/storage/file/{}", self.api_base_url, uuid),
                }
            })
            .collect();
        
        // 7. 计算分页信息
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        let has_more = page < total_pages;
        
        Ok(FileListResponse {
            files,
            total,
            page,
            page_size: limit,
            total_pages,
            has_more,
        })
    }
    
    /// 从file_key中提取原始文件名
    fn extract_filename_from_key(file_key: &str) -> String {
        // file_key格式: user_id/type/timestamp_hash_filename.ext
        // 提取最后的filename部分
        file_key
            .split('/')
            .last()
            .and_then(|s| {
                // 去除 timestamp_hash_ 前缀
                let parts: Vec<&str> = s.splitn(3, '_').collect();
                parts.get(2).map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// 数据库文件记录结构
#[derive(sqlx::FromRow)]
pub struct FileRecord {
    #[sqlx(rename = "file-key")]
    pub file_key: String,
    #[sqlx(rename = "owner-id")]
    pub owner_id: String,
    #[sqlx(rename = "file-type")]
    pub file_type: String,
    #[sqlx(rename = "storage-location")]
    pub storage_location: String,
    #[sqlx(rename = "related-id")]
    pub related_id: Option<String>,
    #[sqlx(rename = "file-size")]
    pub file_size: i64,
    #[sqlx(rename = "content-type")]
    pub content_type: String,
    #[sqlx(rename = "file-hash")]
    pub file_hash: String,
    #[sqlx(rename = "upload-token")]
    pub upload_token: Option<String>,
    #[sqlx(rename = "multipart-upload-id")]
    pub multipart_upload_id: Option<String>,
    pub status: String,
    #[sqlx(rename = "preview-support")]
    pub preview_support: String,
}

impl FileRecord {
    pub fn preview_support(&self) -> PreviewSupport {
        if self.preview_support == "inline_preview" {
            PreviewSupport::InlinePreview
        } else {
            PreviewSupport::DownloadOnly
        }
    }
}

/// UUID映射记录结构
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UuidMappingRecord {
    uuid: String,
    #[sqlx(rename = "physical-file-key")]
    physical_file_key: String,
    #[sqlx(rename = "file-hash")]
    file_hash: String,
    #[sqlx(rename = "file-size")]
    file_size: i64,
    #[sqlx(rename = "content-type")]
    content_type: String,
    #[sqlx(rename = "preview-support")]
    preview_support: String,
    #[sqlx(rename = "first-uploader-id")]
    first_uploader_id: String,
    #[sqlx(rename = "created-at")]
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 权限记录结构
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PermissionRecord {
    id: uuid::Uuid,
    #[sqlx(rename = "access-type")]
    access_type: String,
    #[sqlx(rename = "granted-at")]
    granted_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(rename = "revoked-at")]
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}
