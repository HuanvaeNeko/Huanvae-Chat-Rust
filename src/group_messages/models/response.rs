//! 群消息响应模型

use serde::Serialize;
use super::GroupMessageInfo;

/// 发送消息响应
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_uuid: String,
    pub send_time: String,
}

/// 群消息列表响应
#[derive(Debug, Serialize)]
pub struct GroupMessagesResponse {
    pub messages: Vec<GroupMessageInfo>,
    pub has_more: bool,
}

/// 操作成功响应
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
        }
    }
}

