use crate::common::AppError;
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
    ) -> Result<(), AppError> {
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
    pub async fn is_blacklisted(&self, jti: &str) -> Result<bool, AppError> {
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
    pub async fn enable_blacklist_check(&self, user_id: &str) -> Result<(), AppError> {
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
    /// 返回 (总记录数, 清理数量, 剩余数量)
    pub async fn cleanup_expired_tokens(&self) -> Result<(u64, u64, u64), AppError> {
        // 先查询总记录数
        let total: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM "token-blacklist""#)
            .fetch_one(&self.db)
            .await?;
        let total = total.0 as u64;

        let result = sqlx::query(
            r#"
            DELETE FROM "token-blacklist"
            WHERE "expires-at" < $1
            "#,
        )
        .bind(Utc::now().naive_utc())
        .execute(&self.db)
        .await?;

        let deleted = result.rows_affected();
        let remaining = total.saturating_sub(deleted);

        Ok((total, deleted, remaining))
    }

    /// 清理过期的黑名单检查标识（定时任务调用）
    /// 返回 (总待检查用户数, 重置数量, 剩余待检查数量)
    pub async fn cleanup_expired_checks(&self) -> Result<(u64, u64, u64), AppError> {
        // 先查询需要检查的用户总数
        let total: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "users" WHERE "need-blacklist-check" = true"#,
        )
        .fetch_one(&self.db)
        .await?;
        let total = total.0 as u64;

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

        let reset = result.rows_affected();
        let remaining = total.saturating_sub(reset);

        Ok((total, reset, remaining))
    }

    /// 清理过期的 Access Token 缓存（定时任务调用）
    /// 删除 exp < now() 的记录
    /// 返回 (总记录数, 清理数量, 剩余数量)
    pub async fn cleanup_expired_access_cache(&self) -> Result<(u64, u64, u64), AppError> {
        // 先查询总记录数
        let total: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM "user-access-cache""#)
            .fetch_one(&self.db)
            .await?;
        let total = total.0 as u64;

        let result = sqlx::query(
            r#"
            DELETE FROM "user-access-cache"
            WHERE "exp" < $1
            "#,
        )
        .bind(Utc::now().naive_utc())
        .execute(&self.db)
        .await?;

        let deleted = result.rows_affected();
        let remaining = total.saturating_sub(deleted);

        Ok((total, deleted, remaining))
    }

    /// 批量拉黑用户所有 Access Token（密码修改时调用）
    /// 从 user-access-cache 读取所有未过期的 Token 并加入黑名单
    ///
    /// 优化：使用批量插入替代循环单条插入，避免 N+1 查询问题
    pub async fn blacklist_all_user_access_tokens(
        &self,
        user_id: &str,
        reason: &str,
    ) -> Result<u64, AppError> {
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

        // 批量写入黑名单（使用单条 SQL 批量插入，避免 N+1 问题）
        if !tokens.is_empty() {
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                r#"INSERT INTO "token-blacklist" ("jti", "user-id", "token-type", "expires-at", "reason") "#
            );
            
            query_builder.push_values(tokens.iter(), |mut b, (jti, exp)| {
                b.push_bind(jti)
                 .push_bind(user_id)
                 .push_bind("access")
                 .push_bind(*exp)
                 .push_bind(reason);
            });
            
            query_builder.push(r#" ON CONFLICT ("jti") DO NOTHING"#);
            
            query_builder
                .build()
                .execute(&self.db)
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

