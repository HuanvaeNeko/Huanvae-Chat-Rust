// 主应用模块

pub mod app_state;
pub mod auth;
pub mod common;
pub mod config;
pub mod friends;
pub mod friends_messages;
pub mod groups;
pub mod group_messages;
pub mod profile;
pub mod storage;
pub mod websocket;

// 重导出公共类型，方便外部使用
pub use app_state::AppState;
pub use common::{generate_conversation_uuid, ApiResponse, AppError};
pub use config::{get_config, message_config, storage_config, token_config, websocket_config, AppConfig};

