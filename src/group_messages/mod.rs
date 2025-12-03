//! 群消息系统模块
//!
//! 提供群聊消息功能，包括：
//! - 发送群消息
//! - 获取群消息列表
//! - 删除群消息（个人）
//! - 撤回群消息

pub mod handlers;
pub mod models;
pub mod services;

pub use handlers::create_group_messages_routes;

