use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};

use super::{
    login::{login_handler, LoginState},
    logout::{logout_handler, LogoutState},
    refresh_token::{refresh_token_handler, RefreshTokenState},
    register::{register_handler, RegisterState},
    revoke_device::{list_devices_handler, revoke_device_handler, DeviceState},
};
use crate::auth::middleware::{auth_guard, AuthState};

/// 创建认证路由
pub fn create_auth_routes(
    register_state: RegisterState,
    login_state: LoginState,
    refresh_state: RefreshTokenState,
    logout_state: LogoutState,
    device_state: DeviceState,
    auth_state: AuthState,
) -> Router {
    // 公开路由（无需认证）
    let public_routes = Router::new()
        .route("/register", post(register_handler))
        .with_state(register_state)
        .route("/login", post(login_handler))
        .with_state(login_state)
        .route("/refresh", post(refresh_token_handler))
        .with_state(refresh_state);

    // 需要认证的路由
    let protected_routes = Router::new()
        .route("/logout", post(logout_handler))
        .with_state(logout_state)
        .route("/devices", get(list_devices_handler))
        .route("/devices/{device_id}", delete(revoke_device_handler))  // Axum 0.8+ 使用 {device_id}
        .with_state(device_state)
        .layer(middleware::from_fn_with_state(auth_state, auth_guard));

    // 合并路由
    Router::new().merge(public_routes).merge(protected_routes)
}

