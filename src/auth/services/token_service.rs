use crate::auth::{
    models::{AccessTokenClaims, CreateRefreshToken, RefreshTokenClaims, TokenResponse},
    utils::{generate_device_id, generate_jti, KeyManager},
};
use crate::common::AppError;
use crate::config::token_config;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, Header, Validation};
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use tokio::time::sleep;

/// Token 服务（生成、验证、刷新）
pub struct TokenService {
    key_manager: KeyManager,
    pub(crate) db: PgPool,  // 允许在 auth 模块内部访问
}

impl TokenService {
    /// 创建新的 TokenService
    pub fn new(key_manager: KeyManager, db: PgPool) -> Self {
        Self { key_manager, db }
    }

    /// 生成 Token 对（Access Token + Refresh Token）
    pub async fn generate_token_pair(
        &self,
        user_id: &str,
        email: &str,
        device_info: Option<String>,
        mac_address: Option<String>,
        ip_address: Option<String>,
    ) -> Result<TokenResponse, AppError> {
        let device_id = generate_device_id();
        let token_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let access_token = self.generate_access_token_and_cache(
            user_id,
            email,
            &device_id,
            device_info.as_deref().unwrap_or("Unknown"),
            mac_address.as_deref().unwrap_or("Unknown"),
        ).await?;

        // 生成 Refresh Token（使用配置的有效期）
        let refresh_token = self.generate_refresh_token(user_id, &device_id, &token_id)?;

        // 保存 Refresh Token 到数据库（使用配置的有效期）
        let config = token_config();
        let expires_at = (now + Duration::seconds(config.refresh_token_ttl as i64)).naive_utc();
        let create_token = CreateRefreshToken {
            token_id,
            user_id: user_id.to_string(),
            refresh_token: refresh_token.clone(),
            device_id,
            device_info,
            ip_address,
            expires_at,
        };

        self.save_refresh_token(&create_token).await?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: config.access_token_ttl as i64,
        })
    }

    pub async fn generate_access_token_and_cache(
        &self,
        user_id: &str,
        email: &str,
        device_id: &str,
        device_info: &str,
        mac_address: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let config = token_config();
        let expires_at = now + Duration::seconds(config.access_token_ttl as i64);

        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            device_id: device_id.to_string(),
            device_info: device_info.to_string(),
            mac_address: mac_address.to_string(),
            jti: generate_jti(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };

        self.write_access_cache(&claims).await?;

        let token = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            self.key_manager.encoding_key(),
        )?;

        Ok(token)
    }

    pub async fn write_access_cache(&self, claims: &AccessTokenClaims) -> Result<(), AppError> {
        for attempt in 0..2 {
            let res = sqlx::query(
                r#"
                INSERT INTO "user-access-cache" ("jti", "user-id", "device-id", "exp", "issued-at")
                VALUES ($1, $2, $3, to_timestamp($4), to_timestamp($5))
                ON CONFLICT ("jti") DO NOTHING
                "#,
            )
            .bind(&claims.jti)
            .bind(&claims.sub)
            .bind(&claims.device_id)
            .bind(claims.exp)
            .bind(claims.iat)
            .execute(&self.db)
            .await;

            match res {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if matches!(e, sqlx::Error::PoolTimedOut) && attempt == 0 {
                        sleep(StdDuration::from_millis(100)).await;
                        continue;
                    }
                    return Err(AppError::Database(e.to_string()));
                }
            }
        }
        Ok(())
    }

    /// 生成 Refresh Token（使用配置的有效期）
    fn generate_refresh_token(
        &self,
        user_id: &str,
        device_id: &str,
        token_id: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let config = token_config();
        let expires_at = now + Duration::seconds(config.refresh_token_ttl as i64);

        let claims = RefreshTokenClaims {
            sub: user_id.to_string(),
            device_id: device_id.to_string(),
            token_id: token_id.to_string(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            self.key_manager.encoding_key(),
        )?;

        Ok(token)
    }

    /// 验证并解析 Access Token
    pub fn verify_access_token(&self, token: &str) -> Result<AccessTokenClaims, AppError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<AccessTokenClaims>(
            token,
            self.key_manager.decoding_key(),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    /// 验证并解析 Refresh Token
    pub fn verify_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, AppError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<RefreshTokenClaims>(
            token,
            self.key_manager.decoding_key(),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    /// 刷新 Token（用 Refresh Token 换取新的 Access Token）
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<String, AppError> {
        // 验证 Refresh Token
        let claims = self.verify_refresh_token(refresh_token)?;

        // 从数据库查询 Refresh Token
        let db_token: Option<crate::auth::models::RefreshToken> = sqlx::query_as(
            r#"
            SELECT * FROM "user-refresh-tokens"
            WHERE "token-id" = $1 AND "is-revoked" = false
            "#,
        )
        .bind(&claims.token_id)
        .fetch_optional(&self.db)
        .await?;

        let db_token = db_token.ok_or(AppError::InvalidToken)?;

        // 检查是否过期
        if db_token.expires_at < Utc::now().naive_utc() {
            return Err(AppError::InvalidToken);
        }

        // 更新最后使用时间
        sqlx::query(
            r#"
            UPDATE "user-refresh-tokens"
            SET "last-used-at" = $1
            WHERE "token-id" = $2
            "#,
        )
        .bind(Utc::now())
        .bind(&claims.token_id)
        .execute(&self.db)
        .await?;

        // 查询用户信息
        let user: crate::auth::models::User = sqlx::query_as(
            r#"SELECT * FROM "users" WHERE "user-id" = $1"#,
        )
        .bind(&claims.sub)
        .fetch_one(&self.db)
        .await?;

        let access_token = self.generate_access_token_and_cache(
            &user.user_id,
            &user.user_email,
            &claims.device_id,
            db_token.device_info.as_deref().unwrap_or("Unknown"),
            "Unknown",
        ).await?;

        Ok(access_token)
    }

    /// 保存 Refresh Token 到数据库
    async fn save_refresh_token(&self, token: &CreateRefreshToken) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO "user-refresh-tokens" (
                "token-id", "user-id", "refresh-token", "device-id",
                "device-info", "ip-address", "expires-at"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&token.token_id)
        .bind(&token.user_id)
        .bind(&token.refresh_token)
        .bind(&token.device_id)
        .bind(&token.device_info)
        .bind(&token.ip_address)
        .bind(token.expires_at)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 撤销 Refresh Token（登出时调用）
    pub async fn revoke_refresh_token(
        &self,
        token_id: &str,
        reason: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE "user-refresh-tokens"
            SET "is-revoked" = true, "revoked-at" = $1, "revoked-reason" = $2
            WHERE "token-id" = $3
            "#,
        )
        .bind(Utc::now())
        .bind(reason)
        .bind(token_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// 撤销用户所有设备的 Refresh Token（修改密码时调用）
    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE "user-refresh-tokens"
            SET "is-revoked" = true, "revoked-at" = $1, "revoked-reason" = $2
            WHERE "user-id" = $3 AND "is-revoked" = false
            "#,
        )
        .bind(Utc::now())
        .bind("密码已修改")
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

