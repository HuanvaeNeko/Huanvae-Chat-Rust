use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

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
    ) -> Result<FileUploadResponse> {
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
                    mode: UploadMode::OneTimeToken,
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
                });
            }
        }

        // 6. 判断上传模式
        let upload_mode = FileValidator::determine_upload_mode(request.file_size);

        // 7. 根据模式生成上传凭证
        let response = match upload_mode {
            UploadMode::OneTimeToken => {
                self.generate_one_time_token_upload(
                    user_id,
                    &file_key,
                    &request,
                    preview_support,
                ).await?
            }
            UploadMode::PresignedUrl => {
                self.generate_presigned_url_upload(
                    user_id,
                    &file_key,
                    &request,
                    preview_support,
                ).await?
            }
        };

        Ok(response)
    }

    /// 生成一次性Token上传（< 15GB）
    async fn generate_one_time_token_upload(
        &self,
        user_id: &str,
        file_key: &str,
        request: &FileUploadRequest,
        preview_support: PreviewSupport,
    ) -> Result<FileUploadResponse> {
        let file_hash = request.file_hash.as_deref().unwrap_or("no_hash");
        
        // 生成一次性Token
        let upload_token = Self::generate_upload_token(file_key, user_id, file_hash);
        
        // 计算有效期
        let expires_in = FileValidator::calculate_expires_in(request.file_size, request.estimated_upload_time);
        
        // 生成上传URL
        let upload_url = format!(
            "{}/api/storage/upload/direct?token={}",
            self.api_base_url,
            upload_token
        );

        // 创建数据库记录
        self.create_pending_file_record(
            file_key,
            user_id,
            &request.file_type,
            &request.storage_location,
            request.related_id.as_deref(),
            request.file_size,
            &request.content_type,
            file_hash,
            &upload_token,
            expires_in,
            &preview_support,
        ).await?;

        Ok(FileUploadResponse {
            mode: UploadMode::OneTimeToken,
            preview_support,
            upload_token: Some(upload_token),
            upload_url: Some(upload_url),
            expires_in: Some(expires_in),
            presigned_url: None,
            multipart_upload_id: None,
            file_key: file_key.to_string(),
            max_file_size: FileValidator::get_max_file_size(&request.file_type),
            instant_upload: false,
            existing_file_url: None,
        })
    }

    /// 生成预签名URL上传（> 15GB）
    async fn generate_presigned_url_upload(
        &self,
        user_id: &str,
        file_key: &str,
        request: &FileUploadRequest,
        preview_support: PreviewSupport,
    ) -> Result<FileUploadResponse> {
        let file_hash = request.file_hash.as_deref().unwrap_or("no_hash");
        
        let expires_in = request.estimated_upload_time
            .ok_or_else(|| anyhow::anyhow!("超大文件必须指定预计上传时间"))?;
        
        let bucket = request.storage_location.to_bucket_name();
        
        // 初始化分片上传
        let upload_id = self.s3_client
            .initiate_multipart_upload(bucket, file_key, &request.content_type)
            .await?;

        // 创建数据库记录
        self.create_pending_file_record(
            file_key,
            user_id,
            &request.file_type,
            &request.storage_location,
            request.related_id.as_deref(),
            request.file_size,
            &request.content_type,
            file_hash,
            "",
            expires_in,
            &preview_support,
        ).await?;

        // 保存upload_id
        sqlx::query(
            "UPDATE file_records SET multipart_upload_id = $1 WHERE file_key = $2"
        )
        .bind(&upload_id)
        .bind(file_key)
        .execute(&self.db)
        .await?;

        Ok(FileUploadResponse {
            mode: UploadMode::PresignedUrl,
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
                let conversation_uuid = Self::generate_conversation_uuid(user_id, friend_id);
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
                // group-file/{group_id}/{type}/{timestamp}_{hash}_{filename}.ext
                let group_id = related_id.unwrap_or("unknown");
                let type_dir = match file_type {
                    FileType::GroupImage => "images",
                    FileType::GroupVideo => "videos",
                    FileType::GroupDocument => "files",
                    _ => "files",
                };
                format!("{}/{}/{}_{}_{}",
                    group_id, type_dir, timestamp, hash_prefix, sanitized_name)
            }
        }
    }

    /// 生成会话UUID（用户ID排序组合）
    fn generate_conversation_uuid(user_id1: &str, user_id2: &str) -> String {
        let mut ids = vec![user_id1, user_id2];
        ids.sort();
        format!("{}_{}", ids[0], ids[1])
    }

    /// 生成一次性上传Token
    fn generate_upload_token(file_key: &str, user_id: &str, file_hash: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(file_key.as_bytes());
        hasher.update(user_id.as_bytes());
        hasher.update(file_hash.as_bytes());
        hasher.update(Utc::now().timestamp().to_string().as_bytes());
        hasher.update(uuid::Uuid::new_v4().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 创建待确认的文件记录
    async fn create_pending_file_record(
        &self,
        file_key: &str,
        owner_id: &str,
        file_type: &FileType,
        storage_location: &StorageLocation,
        related_id: Option<&str>,
        file_size: u64,
        content_type: &str,
        file_hash: &str,
        upload_token: &str,
        expires_in: u32,
        preview_support: &PreviewSupport,
    ) -> Result<()> {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        
        sqlx::query(
            "INSERT INTO file_records 
            (file_key, owner_id, file_type, storage_location, related_id,
             file_size, content_type, file_hash, upload_token, status,
             created_at, expires_at, preview_support)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', NOW(), $10, $11)
            ON CONFLICT (file_key) DO NOTHING"
        )
        .bind(file_key)
        .bind(owner_id)
        .bind(file_type.to_string())
        .bind(storage_location.to_string())
        .bind(related_id)
        .bind(file_size as i64)
        .bind(content_type)
        .bind(file_hash)
        .bind(upload_token)
        .bind(expires_at)
        .bind(preview_support.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 验证并获取上传Token信息
    pub async fn verify_and_get_upload_token(
        &self,
        token: &str,
    ) -> Result<FileRecord> {
        let record = sqlx::query_as::<_, FileRecord>(
            "SELECT * FROM file_records
            WHERE upload_token = $1
              AND status = 'pending'
              AND expires_at > NOW()"
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Token无效或已过期"))?;

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
    ) -> Result<String> {
        // 创建UUID映射
        let file_uuid = self.uuid_mapping_service
            .create_mapping(file_key, actual_hash, file_size, content_type, preview_support, owner_id)
            .await?;
        
        // 授予上传者权限
        self.uuid_mapping_service
            .grant_permission(&file_uuid, owner_id, "owner", "upload")
            .await?;
        
        // 生成UUID访问URL
        let uuid_file_url = format!("http://localhost:8080/api/storage/file/{}", file_uuid);
        
        // 更新file_records
        let result = sqlx::query(
            "UPDATE file_records 
            SET status = 'completed',
                upload_token = NULL,
                file_url = $3,
                file_uuid = $4,
                completed_at = NOW()
            WHERE upload_token = $1
              AND file_hash = $2
              AND status = 'pending'"
        )
        .bind(token)
        .bind(actual_hash)
        .bind(&uuid_file_url)
        .bind(&file_uuid)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Token无效或哈希不匹配"));
        }

        Ok(uuid_file_url)
    }

    /// 获取bucket名称
    pub fn get_bucket_name(&self, storage_location: &StorageLocation) -> &str {
        storage_location.to_bucket_name()
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
    ) -> Result<MultipartPartResponse> {
        // 验证upload_id属于该用户
        let record = sqlx::query_as::<_, FileRecord>(
            "SELECT * FROM file_records
            WHERE file_key = $1
              AND multipart_upload_id = $2
              AND owner_id = $3
              AND status = 'pending'"
        )
        .bind(file_key)
        .bind(upload_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("无效的upload_id"))?;

        let storage_loc: StorageLocation = record.storage_location.parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let bucket = self.get_bucket_name(&storage_loc);
        
        let part_url = self.s3_client
            .generate_presigned_upload_part_url(bucket, file_key, upload_id, part_number, 3600)
            .await?;

        Ok(MultipartPartResponse {
            part_url,
            part_number,
            expires_in: 3600,
        })
    }

    /// 生成文件预签名下载URL（通过UUID访问）
    pub async fn generate_presigned_url(
        &self,
        user_id: &str,
        file_uuid: &str,
        expires_in: u32,
    ) -> Result<PresignedUrlResponse> {
        // 1. 查询UUID映射表获取物理文件信息
        let mapping = sqlx::query!(
            r#"
            SELECT uuid, physical_file_key, file_hash, file_size, content_type,
                   preview_support, first_uploader_id, created_at
            FROM file_uuid_mapping
            WHERE uuid = $1
            "#,
            file_uuid
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("文件不存在"))?;

        // 2. 验证用户权限
        let _permission = sqlx::query!(
            r#"
            SELECT id, access_type, granted_at, revoked_at
            FROM file_access_permissions
            WHERE file_uuid = $1 AND user_id = $2 AND revoked_at IS NULL
            "#,
            file_uuid,
            user_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("无权访问此文件"))?;

        // 3. 根据physical_file_key判断bucket
        // physical_file_key格式: user_id/type/timestamp_hash_filename
        let physical_file_key = &mapping.physical_file_key;
        let bucket = if physical_file_key.contains("/images/") {
            "user-file"
        } else if physical_file_key.contains("/videos/") {
            "user-file"
        } else if physical_file_key.contains("/files/") {
            "user-file"
        } else {
            // 默认使用user-file bucket
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
}

/// 数据库文件记录结构
#[derive(sqlx::FromRow)]
pub struct FileRecord {
    pub file_key: String,
    pub owner_id: String,
    pub file_type: String,
    pub storage_location: String,
    pub related_id: Option<String>,
    pub file_size: i64,
    pub content_type: String,
    pub file_hash: String,
    pub upload_token: Option<String>,
    pub multipart_upload_id: Option<String>,
    pub status: String,
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

