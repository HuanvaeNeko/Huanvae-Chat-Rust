use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

use crate::common::AppError;
use crate::storage::client::S3Client;
use crate::storage::models::{ExistingFileInfo, FileType, StorageLocation, PreviewSupport};
use crate::storage::services::UuidMappingService;

/// 去重服务
pub struct DeduplicationService {
    db: PgPool,
    s3_client: Arc<S3Client>,
    uuid_mapping_service: Arc<UuidMappingService>,
}

impl DeduplicationService {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>) -> Self {
        let uuid_mapping_service = Arc::new(UuidMappingService::new(db.clone()));
        Self { 
            db, 
            s3_client,
            uuid_mapping_service,
        }
    }

    /// 检查文件哈希并创建UUID映射引用（秒传核心）
    pub async fn check_and_create_uuid_reference(
        &self,
        file_hash: &str,
        user_id: &str,
        _file_type: &FileType,
        storage_location: &StorageLocation,
        related_id: Option<&str>,
        new_file_key: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &PreviewSupport,
    ) -> Result<Option<ExistingFileInfo>, AppError> {
        // 查询是否存在相同哈希的文件
        if let Some(mapping) = self.uuid_mapping_service.find_by_hash(file_hash).await? {
            // 验证MinIO中物理文件确实存在
            let bucket = Self::extract_bucket_from_key(&mapping.physical_file_key);
            if self.s3_client.file_exists(bucket, &mapping.physical_file_key).await? {
                // 授予当前用户访问权限
                self.uuid_mapping_service
                    .grant_permission(&mapping.uuid, user_id, "owner", "upload")
                    .await?;

                // 群文件秒传：授予所有活跃群成员读取权限
                if *storage_location == StorageLocation::GroupFiles {
                    if let Some(group_id_str) = related_id {
                        info!("群文件秒传，授权群 {} 所有成员访问", group_id_str);
                        let members: Vec<(String,)> = sqlx::query_as(
                            r#"SELECT "user-id" FROM "group-members" 
                               WHERE "group-id" = $1::uuid AND "status" = 'active'"#
                        )
                        .bind(group_id_str)
                        .fetch_all(&self.db)
                        .await?;

                        for (member_id,) in members {
                            if member_id != user_id {
                                self.uuid_mapping_service
                                    .grant_permission(&mapping.uuid, &member_id, "read", "group_share")
                                    .await?;
                            }
                        }
                    }
                }

                // 好友文件秒传：授予好友读取权限
                if *storage_location == StorageLocation::FriendMessages {
                    if let Some(friend_id) = related_id {
                        info!("好友文件秒传，授权好友 {} 访问", friend_id);
                        self.uuid_mapping_service
                            .grant_permission(&mapping.uuid, friend_id, "read", "friend_share")
                            .await?;
                    }
                }

                // 创建file-records记录
                sqlx::query(
                    r#"INSERT INTO "file-records" 
                    ("file-key", "owner-id", "file-type", "storage-location", "related-id",
                     "file-size", "content-type", "file-hash", "physical-file-key", "file-uuid", "status",
                     "file-url", "preview-support", "created-at", "completed-at", "expires-at")
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'completed', $11, $12, NOW(), NOW(), NOW() + INTERVAL '1 year')
                    ON CONFLICT ("file-key") DO NOTHING"#
                )
                .bind(new_file_key)
                .bind(user_id)
                .bind(_file_type.to_string())
                .bind(storage_location.to_string())
                .bind(related_id)
                .bind(file_size)
                .bind(content_type)
                .bind(file_hash)
                .bind(&mapping.physical_file_key)
                .bind(&mapping.uuid)
                .bind(format!("http://localhost:8080/api/storage/file/{}", mapping.uuid))  // 使用UUID访问URL
                .bind(preview_support.to_string())
                .execute(&self.db)
                .await?;

                return Ok(Some(ExistingFileInfo {
                    file_key: new_file_key.to_string(),
                    file_url: format!("http://localhost:8080/api/storage/file/{}", mapping.uuid),
                    file_size: mapping.file_size,
                    content_type: mapping.content_type,
                }));
            }
        }

        Ok(None)
    }

    /// 从file_key提取bucket名称
    fn extract_bucket_from_key(file_key: &str) -> &str {
        if file_key.starts_with("conv-") {
            // 好友文件路径格式: conv-{user1}-{user2}/...
            "friends-file"
        } else if file_key.starts_with("group-") {
            // 群文件路径格式: group-{group_id}/...
            "group-file"
        } else if file_key.contains('/') {
            // 用户个人文件路径格式: {user_id}/...
            "user-file"
        } else {
            // 头像文件格式: {user_id}.{ext}
            "avatars"
        }
    }
}
