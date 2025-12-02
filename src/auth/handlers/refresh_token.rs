use crate::auth::{models::RefreshTokenRequest, services::TokenService};
use crate::common::AppError;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

/// 刷新 Token 处理器状态
#[derive(Clone)]
pub struct RefreshTokenState {
    pub token_service: Arc<TokenService>,
}

/// 刷新 Access Token
pub async fn refresh_token_handler(
    State(state): State<RefreshTokenState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<Value>, AppError> {
    // 使用 Refresh Token 换取新的 Access Token
    let access_token = state
        .token_service
        .refresh_access_token(&req.refresh_token)
        .await?;

    tracing::info!("✅ Token 刷新成功");

    Ok(Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": 900  // 15分钟
    })))
}

