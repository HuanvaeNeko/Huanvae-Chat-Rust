//! 文件下载服务
//!
//! 负责处理文件下载和预签名URL生成

use sqlx::PgPool;
use std::sync::Arc;

use crate::common::AppError;
use crate::storage::client::S3Client;
use crate::storage::models::*;

/// UUID映射记录结构
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct UuidMappingRecord {
    pub uuid: String,
    #[sqlx(rename = "physical-file-key")]
    pub physical_file_key: String,
    #[sqlx(rename = "file-hash")]
    pub file_hash: String,
    #[sqlx(rename = "file-size")]
    pub file_size: i64,
    #[sqlx(rename = "content-type")]
    pub content_type: String,
    #[sqlx(rename = "preview-support")]
    pub preview_support: String,
    #[sqlx(rename = "first-uploader-id")]
    pub first_uploader_id: String,
    #[sqlx(rename = "created-at")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 权限记录结构
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct PermissionRecord {
    pub id: uuid::Uuid,
    #[sqlx(rename = "access-type")]
    pub access_type: String,
    #[sqlx(rename = "granted-at")]
    pub granted_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(rename = "revoked-at")]
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 文件下载服务
pub struct FileDownloadService {
    db: PgPool,
    s3_client: Arc<S3Client>,
}

impl FileDownloadService {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>) -> Self {
        Self { db, s3_client }
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
        let physical_file_key = &mapping.physical_file_key;
        let bucket = if physical_file_key.starts_with("conv-") {
            "friends-file"
        } else if physical_file_key.starts_with("group-") {
            "group-file"
        } else {
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

