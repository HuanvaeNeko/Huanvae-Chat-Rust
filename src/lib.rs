// 主应用模块

pub mod app_state;
pub mod auth;
pub mod common;
pub mod config;
pub mod friends;
pub mod friends_messages;
pub mod profile;
pub mod storage;

// 重导出公共类型，方便外部使用
pub use app_state::AppState;
pub use common::{generate_conversation_uuid, ApiResponse, AppError};
pub use config::{get_config, message_config, storage_config, token_config, AppConfig};

// 可以在这里添加其他模块
// pub mod chat;
// pub mod group;
// pub mod file;

