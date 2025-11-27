use crate::auth::utils::password::hash_password;
use crate::profile::models::{ProfileResponse, UpdatePasswordRequest, UpdateProfileRequest};
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::PgPool;
use tracing::{error, info};

/// 用户资料服务
#[derive(Clone)]
pub struct ProfileService {
    db: PgPool,
}

impl ProfileService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 获取用户完整信息（不含密码）
    pub async fn get_profile(
        &self,
        user_id: &str,
    ) -> Result<ProfileResponse, anyhow::Error> {
        let pool = &self.db;
        let record: (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<NaiveDateTime>, Option<NaiveDateTime>) = sqlx::query_as(
            r#"
            SELECT 
                "user-id",
                "user-nickname",
                "user-email",
                "user-signature",
                "user-avatar-url",
                "admin",
                "created-at",
                "updated-at"
            FROM "users"
            WHERE "user-id" = $1
            "#
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch user profile: {}", e);
            anyhow::anyhow!("User not found")
        })?;

        Ok(ProfileResponse {
            user_id: record.0,
            user_nickname: record.1,
            user_email: record.2,
            user_signature: record.3,
            user_avatar_url: record.4,
            admin: record.5.unwrap_or_else(|| "false".to_string()),
            created_at: record.6.map(|dt| dt.and_utc()).unwrap_or_else(|| DateTime::<Utc>::default()),
            updated_at: record.7.map(|dt| dt.and_utc()).unwrap_or_else(|| DateTime::<Utc>::default()),
        })
    }

    /// 更新个人信息（邮箱、签名）
    pub async fn update_profile(
        &self,
        user_id: &str,
        request: UpdateProfileRequest,
    ) -> Result<(), anyhow::Error> {
        let pool = &self.db;
        // 动态构建更新语句
        let mut updates = Vec::new();
        let mut args_count = 1;

        if request.email.is_some() {
            updates.push(format!(r#""user-email" = ${}"#, args_count));
            args_count += 1;
        }

        if request.signature.is_some() {
            updates.push(format!(r#""user-signature" = ${}"#, args_count));
            args_count += 1;
        }

        if updates.is_empty() {
            return Err(anyhow::anyhow!("No fields to update"));
        }

        let update_clause = updates.join(", ");
        let query_str = format!(
            r#"UPDATE "users" SET {} WHERE "user-id" = ${}"#,
            update_clause, args_count
        );

        let mut query = sqlx::query(&query_str);

        if let Some(email) = &request.email {
            query = query.bind(email);
        }

        if let Some(signature) = &request.signature {
            query = query.bind(signature);
        }

        query = query.bind(user_id);

        query.execute(pool).await.map_err(|e| {
            error!("Failed to update user profile: {}", e);
            anyhow::anyhow!("Failed to update profile")
        })?;

        info!("User profile updated: {}", user_id);
        Ok(())
    }

    /// 更新密码
    pub async fn update_password(
        &self,
        user_id: &str,
        request: UpdatePasswordRequest,
    ) -> Result<(), anyhow::Error> {
        let pool = &self.db;
        // 获取当前密码哈希
        let (current_hash,): (Option<String>,) = sqlx::query_as(
            r#"SELECT "user-password" FROM "users" WHERE "user-id" = $1"#
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch user: {}", e);
            anyhow::anyhow!("User not found")
        })?;

        let current_hash = current_hash.unwrap_or_default();

        // 验证旧密码
        if !bcrypt::verify(&request.old_password, &current_hash).unwrap_or(false) {
            return Err(anyhow::anyhow!("Old password is incorrect"));
        }

        // 哈希新密码
        let new_hash = hash_password(&request.new_password)?;

        // 更新密码
        sqlx::query(
            r#"UPDATE "users" SET "user-password" = $1 WHERE "user-id" = $2"#
        )
        .bind(&new_hash)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to update password: {}", e);
            anyhow::anyhow!("Failed to update password")
        })?;

        info!("Password updated for user: {}", user_id);
        Ok(())
    }

    /// 更新头像 URL
    pub async fn update_avatar_url(
        &self,
        user_id: &str,
        avatar_url: &str,
    ) -> Result<(), anyhow::Error> {
        let pool = &self.db;
        sqlx::query(
            r#"UPDATE "users" SET "user-avatar-url" = $1 WHERE "user-id" = $2"#
        )
        .bind(avatar_url)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to update avatar URL: {}", e);
            anyhow::anyhow!("Failed to update avatar")
        })?;

        info!("Avatar URL updated for user: {}", user_id);
        Ok(())
    }
}

