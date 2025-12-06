//! 文件上传服务
//!
//! 负责处理文件上传相关的所有逻辑

use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

use crate::common::{generate_conversation_uuid, AppError};
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::{DeduplicationService, FileValidator, UuidMappingService};

use super::file_service::FileRecord;

/// 文件上传服务
pub struct FileUploadService {
    db: PgPool,
    s3_client: Arc<S3Client>,
    dedup_service: Arc<DeduplicationService>,
    uuid_mapping_service: Arc<UuidMappingService>,
    api_base_url: String,
}

impl FileUploadService {
    pub fn new(
        db: PgPool,
        s3_client: Arc<S3Client>,
        dedup_service: Arc<DeduplicationService>,
        uuid_mapping_service: Arc<UuidMappingService>,
        api_base_url: String,
    ) -> Self {
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
                    mode: UploadMode::PresignedPut,
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
        
        let expires_in = FileValidator::calculate_expires_in(request.file_size, request.estimated_upload_time);
        
        let presigned_url = self.s3_client
            .generate_presigned_upload_url(bucket, file_key, &request.content_type, expires_in)
            .await?;

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
        
        let upload_id = self.s3_client
            .initiate_multipart_upload(bucket, file_key, &request.content_type)
            .await?;

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
                format!("{}.{}", user_id, extension)
            }
            StorageLocation::UserFiles => {
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

    /// 创建待确认的文件记录
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

    /// 验证并完成预签名上传
    pub async fn verify_and_complete_presigned_upload(
        &self,
        file_key: &str,
        user_id: &str,
    ) -> Result<FileRecord, AppError> {
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

        let storage_loc: StorageLocation = record.storage_location.parse()
            .map_err(|e: String| AppError::BadRequest(e))?;
        let bucket = storage_loc.to_bucket_name();
        
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
        let file_uuid = self.uuid_mapping_service
            .create_mapping(file_key, file_hash, file_size, content_type, preview_support, owner_id)
            .await?;
        
        self.uuid_mapping_service
            .grant_permission(&file_uuid, owner_id, "owner", "upload")
            .await?;
        
        if storage_location == "friend_messages" {
            if let Some(friend_id) = related_id {
                info!("好友文件上传完成，授权好友 {} 访问", friend_id);
                self.uuid_mapping_service
                    .grant_permission(&file_uuid, friend_id, "read", "friend_share")
                    .await?;
            }
        }

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
        
        let uuid_file_url = format!("{}/api/storage/file/{}", self.api_base_url, file_uuid);
        
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
        let record: Option<(String, Option<String>)> = sqlx::query_as(
            r#"SELECT "storage-location", "related-id" FROM "file-records" 
            WHERE "upload-token" = $1 AND "status" = 'pending'"#
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?;

        let (storage_location, related_id) = record
            .ok_or_else(|| AppError::BadRequest("Token无效或已过期".to_string()))?;

        let file_uuid = self.uuid_mapping_service
            .create_mapping(file_key, actual_hash, file_size, content_type, preview_support, owner_id)
            .await?;
        
        self.uuid_mapping_service
            .grant_permission(&file_uuid, owner_id, "owner", "upload")
            .await?;
        
        if storage_location == "friend_messages" {
            if let Some(ref friend_id) = related_id {
                info!("好友文件上传完成，授权好友 {} 访问", friend_id);
                self.uuid_mapping_service
                    .grant_permission(&file_uuid, friend_id, "read", "friend_share")
                    .await?;
            }
        }

        if storage_location == "group_files" {
            if let Some(ref group_id_str) = related_id {
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
        
        let uuid_file_url = format!("{}/api/storage/file/{}", self.api_base_url, file_uuid);
        
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

    /// 生成分片上传URL
    pub async fn generate_multipart_part_url(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        user_id: &str,
    ) -> Result<MultipartPartResponse, AppError> {
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
        let bucket = storage_loc.to_bucket_name();
        
        let multipart_ttl = crate::config::storage_config().multipart_url_ttl;
        let part_url = self.s3_client
            .generate_presigned_upload_part_url(bucket, file_key, upload_id, part_number, multipart_ttl)
            .await?;

        Ok(MultipartPartResponse {
            part_url,
            part_number,
            expires_in: multipart_ttl,
        })
    }
}

