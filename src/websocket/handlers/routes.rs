//! WebSocket 路由配置

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use tracing::{error, info};

use crate::websocket::handlers::{connection::handle_socket, WsState};

/// WebSocket 连接查询参数
#[derive(Debug, Deserialize)]
pub struct WsConnectParams {
    /// Access Token
    pub token: String,
}

/// WebSocket 连接处理器
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsConnectParams>,
    State(state): State<WsState>,
) -> impl IntoResponse {
    // 验证 Token
    let claims = match state.token_service.verify_access_token(&params.token) {
        Ok(claims) => claims,
        Err(e) => {
            error!("WebSocket auth failed: {}", e);
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                "Invalid or expired token",
            )
                .into_response();
        }
    };

    info!(
        user_id = %claims.sub,
        device_id = %claims.device_id,
        "WebSocket upgrade request"
    );

    // 升级到 WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, claims, state))
}

/// 获取 WebSocket 状态（调试用）
async fn ws_status(State(state): State<WsState>) -> impl IntoResponse {
    let status = serde_json::json!({
        "online_users": state.connection_manager.online_user_count(),
        "total_connections": state.connection_manager.total_connection_count(),
    });

    axum::Json(status)
}

/// 创建 WebSocket 路由
pub fn ws_routes(state: WsState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/ws/status", get(ws_status))
        .with_state(state)
}

