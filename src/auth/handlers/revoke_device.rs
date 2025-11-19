use crate::auth::{
    errors::AuthError,
    middleware::AuthContext,
    models::DeviceListResponse,
    services::{BlacklistService, DeviceService},
};
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
) -> Result<Json<DeviceListResponse>, AuthError> {
    // 从请求中提取认证上下文
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AuthError::Unauthorized)?;

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
) -> Result<Json<Value>, AuthError> {
    // 从请求中提取认证上下文
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(AuthError::Unauthorized)?;

    // 撤销设备
    state
        .device_service
        .revoke_device(&auth_context.user_id, &device_id)
        .await?;

    // 启用黑名单检查（15分钟）
    state
        .blacklist_service
        .enable_blacklist_check(&auth_context.user_id)
        .await?;

    tracing::info!("✅ 设备已撤销: {} (用户: {})", device_id, auth_context.user_id);

    Ok(Json(json!({
        "message": "设备已撤销"
    })))
}

