use serde::Deserialize;
use validator::Validate;

/// 发送消息请求
#[derive(Debug, Deserialize, Validate)]
pub struct SendMessageRequest {
    #[validate(length(min = 1))]
    pub receiver_id: String,
    
    #[validate(length(min = 1, max = 10000))]
    pub message_content: String,
    
    #[validate(length(min = 1))]
    pub message_type: String,  // text/image/video/file
    
    pub file_uuid: Option<String>,  // 文件UUID（优先使用）
    pub file_url: Option<String>,   // 文件URL（兼容保留）
    pub file_size: Option<i64>,
}

/// 获取消息列表请求
#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    pub friend_id: String,
    /// 分页：从指定时间戳之前查询（ISO 8601 格式）
    pub before_time: Option<String>,
    /// 默认50，最大500
    pub limit: Option<i32>,
}

/// 删除消息请求
#[derive(Debug, Deserialize, Validate)]
pub struct DeleteMessageRequest {
    #[validate(length(min = 1))]
    pub message_uuid: String,
}

/// 撤回消息请求
#[derive(Debug, Deserialize, Validate)]
pub struct RecallMessageRequest {
    #[validate(length(min = 1))]
    pub message_uuid: String,
}
