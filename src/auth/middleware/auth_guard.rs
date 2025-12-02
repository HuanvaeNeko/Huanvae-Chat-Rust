use crate::auth::{
    models::AccessTokenClaims,
    services::{BlacklistService, TokenService},
};
use crate::common::AppError;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::{NaiveDateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;

/// 认证中间件状态
#[derive(Clone)]
pub struct AuthState {
    pub token_service: Arc<TokenService>,
    pub blacklist_service: Arc<BlacklistService>,
    pub db: PgPool,
}

/// 认证上下文（注入到请求中）
#[derive(Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub email: String,
    pub device_id: String,
    pub claims: AccessTokenClaims,
}

/// 认证中间件（验证 Access Token + 黑名单检查）
pub async fn auth_guard(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 从请求头提取 Token
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // 2. 验证 Bearer 格式
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidToken)?;

    // 3. 验证并解析 Access Token
    let claims = state.token_service.verify_access_token(token)?;

    // 4. 查询用户的黑名单检查标识
    let user: (bool, Option<NaiveDateTime>) = sqlx::query_as(
        r#"
        SELECT "need-blacklist-check", "blacklist-check-expires-at"
        FROM "users"
        WHERE "user-id" = $1
        "#,
    )
    .bind(&claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::warn!("查询用户失败: {}", e);
        AppError::InvalidToken
    })?;

    let (need_check, check_expires_at) = user;

    // 5. 智能黑名单检查
    if need_check {
        // 检查是否已过期
        if let Some(expires_at) = check_expires_at {
            if expires_at < Utc::now().naive_utc() {
                // 过期了，自动关闭检查
                let _ = sqlx::query(
                    r#"
                    UPDATE "users"
                    SET "need-blacklist-check" = false,
                        "blacklist-check-expires-at" = NULL
                    WHERE "user-id" = $1
                    "#,
                )
                .bind(&claims.sub)
                .execute(&state.db)
                .await;
            } else {
                // 未过期，执行黑名单检查
                if state.blacklist_service.is_blacklisted(&claims.jti).await? {
                    return Err(AppError::TokenRevoked);
                }
            }
        }
    }

    // 6. 创建认证上下文并注入到请求中
    let auth_context = AuthContext {
        user_id: claims.sub.clone(),
        email: claims.email.clone(),
        device_id: claims.device_id.clone(),
        claims: claims.clone(),
    };

    request.extensions_mut().insert(auth_context);

    // 7. 继续处理请求
    Ok(next.run(request).await)
}

/// 从请求中提取认证上下文（在 Handler 中使用）
pub fn extract_auth_context(request: &Request) -> Result<AuthContext, AppError> {
    request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AppError::Unauthorized)
}

