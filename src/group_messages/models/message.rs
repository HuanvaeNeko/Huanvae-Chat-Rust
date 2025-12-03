//! 群消息模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 群消息数据库模型
#[derive(Debug, Clone, FromRow)]
pub struct GroupMessage {
    #[sqlx(rename = "message-uuid")]
    pub message_uuid: Uuid,
    #[sqlx(rename = "group-id")]
    pub group_id: Uuid,
    #[sqlx(rename = "sender-id")]
    pub sender_id: String,
    #[sqlx(rename = "message-content")]
    pub message_content: String,
    #[sqlx(rename = "message-type")]
    pub message_type: String,
    #[sqlx(rename = "file-uuid")]
    pub file_uuid: Option<String>,
    #[sqlx(rename = "file-url")]
    pub file_url: Option<String>,
    #[sqlx(rename = "file-size")]
    pub file_size: Option<i64>,
    #[sqlx(rename = "reply-to")]
    pub reply_to: Option<Uuid>,
    #[sqlx(rename = "send-time")]
    pub send_time: DateTime<Utc>,
    #[sqlx(rename = "is-recalled")]
    pub is_recalled: bool,
    #[sqlx(rename = "recalled-at")]
    pub recalled_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "recalled-by")]
    pub recalled_by: Option<String>,
}

/// 群消息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessageInfo {
    pub message_uuid: String,
    pub group_id: String,
    pub sender_id: String,
    pub sender_nickname: Option<String>,
    pub sender_avatar_url: Option<String>,
    pub message_content: String,
    pub message_type: String,
    pub file_uuid: Option<String>,
    pub file_url: Option<String>,
    pub file_size: Option<i64>,
    pub reply_to: Option<String>,
    pub send_time: String,
    pub is_recalled: bool,
}

impl From<GroupMessage> for GroupMessageInfo {
    fn from(m: GroupMessage) -> Self {
        Self {
            message_uuid: m.message_uuid.to_string(),
            group_id: m.group_id.to_string(),
            sender_id: m.sender_id,
            sender_nickname: None,
            sender_avatar_url: None,
            message_content: if m.is_recalled { "[消息已撤回]".to_string() } else { m.message_content },
            message_type: m.message_type,
            file_uuid: if m.is_recalled { None } else { m.file_uuid },
            file_url: if m.is_recalled { None } else { m.file_url },
            file_size: if m.is_recalled { None } else { m.file_size },
            reply_to: m.reply_to.map(|u| u.to_string()),
            send_time: m.send_time.to_rfc3339(),
            is_recalled: m.is_recalled,
        }
    }
}

