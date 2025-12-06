//! TURN 路由定义

use axum::{
    middleware,
    routing::{any, get},
    Router,
};

use super::coordinator_ws::coordinator_ws_handler;
use super::ice_servers::get_ice_servers;
use super::TurnState;
use crate::auth::middleware::{auth_guard, AuthState};

/// 创建 TURN 路由
pub fn turn_routes(turn_state: TurnState, auth_state: AuthState) -> Router {
    // ICE 服务器路由（需要认证）
    let ice_servers_route = Router::new()
        .route("/api/webrtc/ice-servers", get(get_ice_servers))
        .route_layer(middleware::from_fn_with_state(auth_state, auth_guard))
        .with_state(turn_state.clone());

    // Agent WebSocket 路由（使用 token 参数认证）
    let coordinator_route = Router::new()
        .route("/internal/turn-coordinator", any(coordinator_ws_handler))
        .with_state(turn_state);

    // 合并路由
    ice_servers_route.merge(coordinator_route)
}
