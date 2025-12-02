use crate::auth::errors::AuthError;
use crate::config::token_config;
use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::PgPool;

/// 黑名单服务（Token 撤销管理）
pub struct BlacklistService {
    db: PgPool,
}

impl BlacklistService {
    /// 创建新的 BlacklistService
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 将 Token 添加到黑名单
    pub async fn add_to_blacklist(
        &self,
        jti: &str,
        user_id: &str,
        token_type: &str,
        expires_at: NaiveDateTime,
        reason: Option<String>,
    ) -> Result<(), AuthError> {
        sqlx::query(
            r#"
            INSERT INTO "token-blacklist" ("jti", "user-id", "token-type", "expires-at", "reason")
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT ("jti") DO NOTHING
            "#,
        )
        .bind(jti)
        .bind(user_id)
        .bind(token_type)
        .bind(expires_at)
        .bind(reason)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 检查 Token 是否在黑名单中
    pub async fn is_blacklisted(&self, jti: &str) -> Result<bool, AuthError> {
        let result: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM "token-blacklist"
                WHERE "jti" = $1 AND "expires-at" > $2
            )
            "#,
        )
        .bind(jti)
        .bind(Utc::now().naive_utc())
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|r| r.0).unwrap_or(false))
    }

    /// 启用用户的黑名单检查（使用配置的检查窗口时间）
    pub async fn enable_blacklist_check(&self, user_id: &str) -> Result<(), AuthError> {
        let config = token_config();
        let expires_at = (Utc::now() + Duration::seconds(config.blacklist_check_window as i64)).naive_utc();

        sqlx::query(
            r#"
            UPDATE "users"
            SET "need-blacklist-check" = true,
                "blacklist-check-expires-at" = $1
            WHERE "user-id" = $2
            "#,
        )
        .bind(expires_at)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 清理过期的黑名单记录（定时任务调用）
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, AuthError> {
        let result = sqlx::query(
            r#"
            DELETE FROM "token-blacklist"
            WHERE "expires-at" < $1
            "#,
        )
        .bind(Utc::now().naive_utc())
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected())
    }

    /// 清理过期的黑名单检查标识（定时任务调用）
    pub async fn cleanup_expired_checks(&self) -> Result<u64, AuthError> {
        let result = sqlx::query(
            r#"
            UPDATE "users"
            SET "need-blacklist-check" = false,
                "blacklist-check-expires-at" = NULL
            WHERE "need-blacklist-check" = true
              AND "blacklist-check-expires-at" < $1
            "#,
        )
        .bind(Utc::now().naive_utc())
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected())
    }

    /// 批量拉黑用户所有 Access Token（密码修改时调用）
    /// 从 user-access-cache 读取所有未过期的 Token 并加入黑名单
    pub async fn blacklist_all_user_access_tokens(
        &self,
        user_id: &str,
        reason: &str,
    ) -> Result<u64, AuthError> {
        // 从 user-access-cache 获取所有未过期的 jti 和过期时间
        let tokens: Vec<(String, NaiveDateTime)> = sqlx::query_as(
            r#"
            SELECT "jti", "exp"
            FROM "user-access-cache"
            WHERE "user-id" = $1 AND "exp" > NOW()
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        let count = tokens.len() as u64;

        // 批量写入黑名单
        for (jti, exp) in tokens {
            self.add_to_blacklist(&jti, user_id, "access", exp, Some(reason.to_string()))
                .await?;
        }

        // 启用黑名单检查（15分钟窗口）
        self.enable_blacklist_check(user_id).await?;

        tracing::info!(
            "🔒 用户 {} 的 {} 个 Access Token 已被拉黑，原因: {}",
            user_id,
            count,
            reason
        );

        Ok(count)
    }
}

