use crate::auth::{
    middleware::AuthContext,
    models::DeviceListResponse,
    services::{BlacklistService, DeviceService},
};
use crate::common::AppError;
use axum::{
    extract::{Path, Request, State},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

/// 设备管理处理器状态
#[derive(Clone)]
pub struct DeviceState {
    pub device_service: Arc<DeviceService>,
    pub blacklist_service: Arc<BlacklistService>,
}

/// 获取用户所有设备列表
pub async fn list_devices_handler(
    State(state): State<DeviceState>,
    request: Request,
) -> Result<Json<DeviceListResponse>, AppError> {
    // 从请求中提取认证上下文
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AppError::Unauthorized)?;

    // 查询设备列表
    let devices = state
        .device_service
        .list_user_devices(&auth_context.user_id, Some(&auth_context.device_id))
        .await?;

    let total = devices.len();

    Ok(Json(DeviceListResponse { devices, total }))
}

/// 撤销指定设备（远程登出）
pub async fn revoke_device_handler(
    State(state): State<DeviceState>,
    Path(device_id): Path<String>,
    request: Request,
) -> Result<Json<Value>, AppError> {
    // 从请求中提取认证上下文
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AppError::Unauthorized)?;

    // 启用黑名单检查（窗口 15 分钟）
    state
        .blacklist_service
        .enable_blacklist_check(&auth_context.user_id)
        .await?;

    // 按设备读取缓存并批量拉黑 Access Token
    let cached = state
        .device_service
        .list_cached_access_tokens(&auth_context.user_id, &device_id)
        .await?;

    for (jti, exp) in cached.iter() {
        state
            .blacklist_service
            .add_to_blacklist(jti, &auth_context.user_id, "access", *exp, Some("远程登出".to_string()))
            .await?;
    }

    if cached.is_empty() && device_id == auth_context.device_id {
        let exp_dt = chrono::DateTime::from_timestamp(auth_context.claims.exp, 0)
            .map(|dt| dt.naive_utc())
            .unwrap_or(chrono::Utc::now().naive_utc());
        state
            .blacklist_service
            .add_to_blacklist(&auth_context.claims.jti, &auth_context.user_id, "access", exp_dt, Some("远程登出(兜底)".to_string()))
            .await?;
    }

    // 撤销设备（Refresh Token）
    state
        .device_service
        .revoke_device(&auth_context.user_id, &device_id)
        .await?;

    tracing::info!("✅ 设备已撤销: {} (用户: {})", device_id, auth_context.user_id);

    Ok(Json(json!({
        "message": "设备已撤销"
    })))
}

