use sqlx::PgPool;
use std::sync::Arc;

use crate::storage::client::S3Client;
use crate::storage::models::{ExistingFileInfo, FileType};

/// 去重服务
pub struct DeduplicationService {
    db: PgPool,
    s3_client: Arc<S3Client>,
}

impl DeduplicationService {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>) -> Self {
        Self { db, s3_client }
    }

    /// 检查文件哈希是否已存在（秒传核心）
    pub async fn check_file_exists_by_hash(
        &self,
        file_hash: &str,
        _user_id: &str,
        _file_type: &FileType,
    ) -> Result<Option<ExistingFileInfo>, anyhow::Error> {
        // 查询是否有相同哈希的已完成文件
        let existing = sqlx::query_as::<_, ExistingFileInfoRow>(
            "SELECT file_key, file_url, file_size, content_type
            FROM file_records
            WHERE file_hash = $1
              AND status = 'completed'
              AND deleted_at IS NULL
              AND file_url IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 1"
        )
        .bind(file_hash)
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = existing {
            // 验证MinIO中文件确实存在
            let bucket = Self::extract_bucket_from_key(&row.file_key);
            if self.s3_client.file_exists(bucket, &row.file_key).await? {
                return Ok(Some(ExistingFileInfo {
                    file_key: row.file_key,
                    file_url: row.file_url,
                    file_size: row.file_size,
                    content_type: row.content_type,
                }));
            }
        }

        Ok(None)
    }

    /// 从file_key提取bucket名称
    fn extract_bucket_from_key(file_key: &str) -> &str {
        if file_key.contains('/') {
            // user-file, friends-file等
            "user-file"
        } else {
            // avatars
            "avatars"
        }
    }
}

#[derive(sqlx::FromRow)]
struct ExistingFileInfoRow {
    file_key: String,
    file_url: String,
    file_size: i64,
    content_type: String,
}

