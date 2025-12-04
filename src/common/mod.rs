//! 公共模块 - 统一错误类型、响应格式和工具函数
//!
//! 提供全局统一的：
//! - `AppError` - 统一错误类型
//! - `ApiResponse` - 统一 API 响应格式
//! - `generate_conversation_uuid` - 生成会话唯一标识
//! - `MessageArchiveService` - 消息归档服务

mod errors;
mod response;
mod message_archive_service;

pub use errors::AppError;
pub use response::ApiResponse;
pub use message_archive_service::MessageArchiveService;

/// 生成会话UUID（双方用户ID排序后组合）
///
/// 将两个用户ID按字母顺序排序后组合，确保双方使用相同的会话标识。
///
/// # Arguments
/// * `user_id_1` - 第一个用户ID
/// * `user_id_2` - 第二个用户ID
///
/// # Returns
/// 格式为 `conv-{sorted_user1}-{sorted_user2}` 的会话UUID
///
/// # Example
/// ```
/// let uuid = generate_conversation_uuid("user-456", "user-123");
/// assert_eq!(uuid, "conv-user-123-user-456");
/// ```
pub fn generate_conversation_uuid(user_id_1: &str, user_id_2: &str) -> String {
    let mut ids = vec![user_id_1, user_id_2];
    ids.sort();
    format!("conv-{}-{}", ids[0], ids[1])
}

