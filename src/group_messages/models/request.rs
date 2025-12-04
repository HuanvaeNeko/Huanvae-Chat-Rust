//! 群消息请求模型

use serde::Deserialize;

/// 发送群消息请求
#[derive(Debug, Deserialize)]
pub struct SendGroupMessageRequest {
    pub group_id: String,
    pub message_content: String,
    pub message_type: Option<String>,
    pub file_uuid: Option<String>,
    pub file_url: Option<String>,
    pub file_size: Option<i64>,
    pub reply_to: Option<String>,
}

/// 获取群消息请求参数
#[derive(Debug, Deserialize)]
pub struct GetGroupMessagesQuery {
    pub group_id: String,
    /// 分页：从指定时间戳之前查询（ISO 8601 格式）
    pub before_time: Option<String>,
    pub limit: Option<i32>,
}

/// 删除群消息请求
#[derive(Debug, Deserialize)]
pub struct DeleteGroupMessageRequest {
    pub message_uuid: String,
}

/// 撤回群消息请求
#[derive(Debug, Deserialize)]
pub struct RecallGroupMessageRequest {
    pub message_uuid: String,
}
