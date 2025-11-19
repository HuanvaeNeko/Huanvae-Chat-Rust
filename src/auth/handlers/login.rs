use crate::auth::{
    errors::AuthError,
    models::{LoginRequest, TokenResponse},
    services::TokenService,
    utils::verify_password,
};
use axum::{extract::State, Json};
use sqlx::PgPool;
use std::sync::Arc;

/// 登录处理器状态
#[derive(Clone)]
pub struct LoginState {
    pub db: PgPool,
    pub token_service: Arc<TokenService>,
}

/// 用户登录
pub async fn login_handler(
    State(state): State<LoginState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AuthError> {
    // 1. 查询用户（按ID）
    let user: Option<crate::auth::models::User> = sqlx::query_as(
        r#"SELECT * FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&req.user_id)
    .fetch_optional(&state.db)
    .await?;

    let user = user.ok_or(AuthError::InvalidCredentials)?;

    // 2. 验证密码
    if !verify_password(&req.password, &user.user_password)? {
        return Err(AuthError::InvalidCredentials);
    }

    // 3. 生成 Token 对（Access Token + Refresh Token）
    let token_response = state
        .token_service
        .generate_token_pair(
            &user.user_id,
            &user.user_email,
            req.device_info,
            req.mac_address,
            None, // IP 地址可以从请求头提取
        )
        .await?;

    tracing::info!("✅ 用户登录成功: {} ({})", user.user_nickname, user.user_email);

    Ok(Json(token_response))
}

