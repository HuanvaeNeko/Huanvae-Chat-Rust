use crate::auth::{
    errors::AuthError,
    middleware::AuthContext,
    services::{BlacklistService, TokenService},
};
use axum::{extract::{Request, State}, Json};
use serde_json::{json, Value};
use std::sync::Arc;

/// 登出处理器状态
#[derive(Clone)]
pub struct LogoutState {
    pub token_service: Arc<TokenService>,
    pub blacklist_service: Arc<BlacklistService>,
}

/// 用户登出（撤销当前设备的 Refresh Token + 将 Access Token 加入黑名单）
pub async fn logout_handler(
    State(state): State<LogoutState>,
    request: Request,
) -> Result<Json<Value>, AuthError> {
    // 1. 从请求中提取认证上下文
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AuthError::Unauthorized)?;

    // 2. 保证 need-blacklist-check 开启（窗口 15 分钟）
    state
        .blacklist_service
        .enable_blacklist_check(&auth_context.user_id)
        .await?;

    // 3. 读取缓存并按设备批量拉黑 Access Token
    let cached: Vec<(String, chrono::NaiveDateTime)> = sqlx::query_as(
        r#"
        SELECT "jti", "exp" FROM "user-access-cache"
        WHERE "user-id" = $1 AND "device-id" = $2 AND "exp" > $3
        "#,
    )
    .bind(&auth_context.user_id)
    .bind(&auth_context.device_id)
    .bind(chrono::Utc::now().naive_utc())
    .fetch_all(&state.token_service.db)
    .await?;

    for (jti, exp) in cached.iter() {
        state
            .blacklist_service
            .add_to_blacklist(jti, &auth_context.user_id, "access", *exp, Some("用户登出".to_string()))
            .await?;
    }

    if cached.is_empty() {
        let exp_dt = chrono::DateTime::from_timestamp(auth_context.claims.exp, 0)
            .map(|dt| dt.naive_utc())
            .unwrap_or(chrono::Utc::now().naive_utc());
        state
            .blacklist_service
            .add_to_blacklist(&auth_context.claims.jti, &auth_context.user_id, "access", exp_dt, Some("用户登出(兜底)".to_string()))
            .await?;
    }

    // 4. 查找当前设备的 Refresh Token
    let refresh_tokens: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT "token-id" FROM "user-refresh-tokens"
        WHERE "user-id" = $1 AND "device-id" = $2 AND "is-revoked" = false
        "#,
    )
    .bind(&auth_context.user_id)
    .bind(&auth_context.device_id)
    .fetch_all(&state.token_service.db)
    .await?;

    // 5. 撤销所有匹配的 Refresh Token
    for (token_id,) in refresh_tokens {
        state
            .token_service
            .revoke_refresh_token(&token_id, Some("用户登出".to_string()))
            .await?;
    }

    tracing::info!("✅ 用户登出成功: {}", auth_context.user_id);

    Ok(Json(json!({
        "message": "登出成功"
    })))
}

