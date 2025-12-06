//! WebRTC 房间路由定义

use axum::{
    middleware,
    routing::{any, post},
    Router,
};

use super::join::join_room;
use super::rooms::create_room;
use super::signaling_ws::signaling_ws_handler;
use super::WebRTCState;
use crate::auth::middleware::{auth_guard, AuthState};

/// 创建 WebRTC 房间路由
pub fn webrtc_room_routes(state: WebRTCState, auth_state: AuthState) -> Router {
    // 需要认证的路由（创建房间）
    let authenticated_routes = Router::new()
        .route("/api/webrtc/rooms", post(create_room))
        .route_layer(middleware::from_fn_with_state(auth_state, auth_guard))
        .with_state(state.clone());

    // 无需认证的路由（加入房间）
    let public_routes = Router::new()
        .route("/api/webrtc/rooms/{room_id}/join", post(join_room))
        .with_state(state.clone());

    // WebSocket 信令路由（使用 token 参数认证）
    let ws_routes = Router::new()
        .route("/ws/webrtc/rooms/{room_id}", any(signaling_ws_handler))
        .with_state(state);

    // 合并所有路由
    authenticated_routes.merge(public_routes).merge(ws_routes)
}

