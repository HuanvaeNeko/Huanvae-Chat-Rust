use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// UUID映射信息
#[derive(Debug, Clone)]
pub struct UuidMappingInfo {
    pub uuid: String,
    pub physical_file_key: String,
    pub file_hash: String,
    pub file_size: i64,
    pub content_type: String,
    pub preview_support: String,
}

/// UUID映射服务
pub struct UuidMappingService {
    db: PgPool,
}

impl UuidMappingService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 通过哈希查找已存在的UUID映射
    pub async fn find_by_hash(&self, file_hash: &str) -> Result<Option<UuidMappingInfo>> {
        let result = sqlx::query_as::<_, UuidMappingRow>(
            r#"SELECT "uuid", "physical-file-key", "file-hash", "file-size", "content-type", "preview-support"
             FROM "file-uuid-mapping"
             WHERE "file-hash" = $1
             LIMIT 1"#
        )
        .bind(file_hash)
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|row| UuidMappingInfo {
            uuid: row.uuid,
            physical_file_key: row.physical_file_key,
            file_hash: row.file_hash,
            file_size: row.file_size,
            content_type: row.content_type,
            preview_support: row.preview_support,
        }))
    }

    /// 创建新的UUID映射
    pub async fn create_mapping(
        &self,
        physical_file_key: &str,
        file_hash: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &str,
        uploader_id: &str,
    ) -> Result<String> {
        let uuid = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO "file-uuid-mapping" 
             ("uuid", "physical-file-key", "file-hash", "file-size", "content-type", "preview-support", "first-uploader-id")
             VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(&uuid)
        .bind(physical_file_key)
        .bind(file_hash)
        .bind(file_size)
        .bind(content_type)
        .bind(preview_support)
        .bind(uploader_id)
        .execute(&self.db)
        .await?;

        Ok(uuid)
    }

    /// 通过UUID获取映射信息
    pub async fn get_by_uuid(&self, uuid: &str) -> Result<Option<UuidMappingInfo>> {
        let result = sqlx::query_as::<_, UuidMappingRow>(
            r#"SELECT "uuid", "physical-file-key", "file-hash", "file-size", "content-type", "preview-support"
             FROM "file-uuid-mapping"
             WHERE "uuid" = $1"#
        )
        .bind(uuid)
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|row| UuidMappingInfo {
            uuid: row.uuid,
            physical_file_key: row.physical_file_key,
            file_hash: row.file_hash,
            file_size: row.file_size,
            content_type: row.content_type,
            preview_support: row.preview_support,
        }))
    }

    /// 授予用户访问权限
    pub async fn grant_permission(
        &self,
        file_uuid: &str,
        user_id: &str,
        access_type: &str,
        granted_by: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO "file-access-permissions" 
             ("file-uuid", "user-id", "access-type", "granted-by")
             VALUES ($1, $2, $3, $4)"#
        )
        .bind(file_uuid)
        .bind(user_id)
        .bind(access_type)
        .bind(granted_by)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 检查用户是否有访问权限
    pub async fn check_permission(&self, file_uuid: &str, user_id: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM "file-access-permissions"
             WHERE "file-uuid" = $1 AND "user-id" = $2 AND "revoked-at" IS NULL"#
        )
        .bind(file_uuid)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result > 0)
    }

    /// 撤销用户访问权限（软删除）
    pub async fn revoke_permission(&self, file_uuid: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            r#"UPDATE "file-access-permissions" 
             SET "revoked-at" = NOW()
             WHERE "file-uuid" = $1 AND "user-id" = $2 AND "revoked-at" IS NULL"#
        )
        .bind(file_uuid)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct UuidMappingRow {
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
}
