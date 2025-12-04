//! WebSocket 实时通信模块
//!
//! 提供实时消息推送、未读消息通知、已读同步等功能。
//! 支持好友私信和群聊消息的统一通知推送。

pub mod handlers;
pub mod models;
pub mod services;

// 重导出常用类型
pub use handlers::routes::ws_routes;
pub use handlers::state::WsState;
pub use services::connection_manager::ConnectionManager;
pub use services::notification_service::NotificationService;
pub use services::unread_service::UnreadService;

