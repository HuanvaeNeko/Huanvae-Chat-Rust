use crate::auth::{errors::AuthError, models::Device};
use sqlx::PgPool;

/// 设备管理服务
pub struct DeviceService {
    db: PgPool,
}

impl DeviceService {
    /// 创建新的 DeviceService
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 获取用户所有设备列表
    pub async fn list_user_devices(
        &self,
        user_id: &str,
        current_device_id: Option<&str>,
    ) -> Result<Vec<Device>, AuthError> {
        let rows: Vec<(String, Option<String>, Option<String>, Option<chrono::NaiveDateTime>, chrono::NaiveDateTime)> = sqlx::query_as(
            r#"
            SELECT "device-id", "device-info", "ip-address", "last-used-at", "created-at"
            FROM "user-refresh-tokens"
            WHERE "user-id" = $1 AND "is-revoked" = false
            ORDER BY "last-used-at" DESC NULLS LAST
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        let devices = rows
            .into_iter()
            .map(|(device_id, device_info, ip_address, last_used_at, created_at)| Device {
                device_id: device_id.clone(),
                device_info: device_info.unwrap_or_else(|| "Unknown".to_string()),
                ip_address,
                last_used_at,
                created_at,
                is_current: current_device_id.map(|id| id == device_id).unwrap_or(false),
            })
            .collect();

        Ok(devices)
    }

    /// 撤销指定设备（删除其 Refresh Token）
    pub async fn revoke_device(&self, user_id: &str, device_id: &str) -> Result<(), AuthError> {
        let result = sqlx::query(
            r#"
            UPDATE "user-refresh-tokens"
            SET "is-revoked" = true,
                "revoked-at" = $1,
                "revoked-reason" = '远程登出'
            WHERE "user-id" = $2 AND "device-id" = $3 AND "is-revoked" = false
            "#,
        )
        .bind(chrono::Utc::now())
        .bind(user_id)
        .bind(device_id)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AuthError::DeviceNotFound);
        }

        Ok(())
    }

    /// 撤销用户所有设备（除了当前设备）
    pub async fn revoke_all_except_current(
        &self,
        user_id: &str,
        current_device_id: &str,
    ) -> Result<u64, AuthError> {
        let result = sqlx::query(
            r#"
            UPDATE "user-refresh-tokens"
            SET "is-revoked" = true,
                "revoked-at" = $1,
                "revoked-reason" = '批量登出'
            WHERE "user-id" = $2 AND "device-id" != $3 AND "is-revoked" = false
            "#,
        )
        .bind(chrono::Utc::now())
        .bind(user_id)
        .bind(current_device_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected())
    }
}

