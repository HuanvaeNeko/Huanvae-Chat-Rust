//! WebSocket 状态管理

use std::sync::Arc;

use crate::auth::services::TokenService;
use crate::websocket::services::{ConnectionManager, NotificationService, UnreadService};

/// WebSocket 模块状态
#[derive(Clone)]
pub struct WsState {
    /// 连接管理器
    pub connection_manager: Arc<ConnectionManager>,
    /// 通知服务
    pub notification_service: NotificationService,
    /// 未读消息服务
    pub unread_service: UnreadService,
    /// Token 服务（用于验证 WebSocket 连接）
    pub token_service: Arc<TokenService>,
}

impl WsState {
    /// 创建 WebSocket 状态
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        notification_service: NotificationService,
        unread_service: UnreadService,
        token_service: Arc<TokenService>,
    ) -> Self {
        Self {
            connection_manager,
            notification_service,
            unread_service,
            token_service,
        }
    }
}

