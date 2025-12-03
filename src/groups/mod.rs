//! 群聊系统模块
//!
//! 提供群聊管理功能，包括：
//! - 创建/解散群聊
//! - 成员管理（邀请、退出、移除）
//! - 角色管理（群主转让、管理员设置）
//! - 禁言功能
//! - 邀请码管理
//! - 群公告管理

pub mod handlers;
pub mod models;
pub mod services;

pub use handlers::create_group_routes;

