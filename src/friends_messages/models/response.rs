use serde::Serialize;
use crate::friends_messages::models::MessageResponse;

/// 发送消息响应
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_uuid: String,
    pub send_time: String,
}

/// 消息列表响应
#[derive(Debug, Serialize)]
pub struct MessagesListResponse {
    pub messages: Vec<MessageResponse>,
    pub has_more: bool,
}

/// 通用成功响应
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

