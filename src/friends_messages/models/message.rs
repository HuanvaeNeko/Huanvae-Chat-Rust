use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum MessageType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "file")]
    File,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Text => write!(f, "text"),
            MessageType::Image => write!(f, "image"),
            MessageType::Video => write!(f, "video"),
            MessageType::File => write!(f, "file"),
        }
    }
}

impl std::str::FromStr for MessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(MessageType::Text),
            "image" => Ok(MessageType::Image),
            "video" => Ok(MessageType::Video),
            "file" => Ok(MessageType::File),
            _ => Err(format!("无效的消息类型: {}", s)),
        }
    }
}

/// 数据库中的消息模型
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    #[serde(rename = "message_uuid")]
    #[sqlx(rename = "message-uuid")]
    pub message_uuid: String,
    
    #[serde(rename = "conversation_uuid")]
    #[sqlx(rename = "conversation-uuid")]
    pub conversation_uuid: String,
    
    #[serde(rename = "sender_id")]
    #[sqlx(rename = "sender-id")]
    pub sender_id: String,
    
    #[serde(rename = "receiver_id")]
    #[sqlx(rename = "receiver-id")]
    pub receiver_id: String,
    
    #[serde(rename = "message_content")]
    #[sqlx(rename = "message-content")]
    pub message_content: String,
    
    #[serde(rename = "message_type")]
    #[sqlx(rename = "message-type")]
    pub message_type: String,
    
    #[serde(rename = "file_url")]
    #[sqlx(rename = "file-url")]
    pub file_url: Option<String>,
    
    #[serde(rename = "file_size")]
    #[sqlx(rename = "file-size")]
    pub file_size: Option<i64>,
    
    #[serde(rename = "send_time")]
    #[sqlx(rename = "send-time")]
    pub send_time: DateTime<Utc>,
    
    #[serde(rename = "is_deleted_by_sender")]
    #[sqlx(rename = "is-deleted-by-sender")]
    pub is_deleted_by_sender: bool,
    
    #[serde(rename = "is_deleted_by_receiver")]
    #[sqlx(rename = "is-deleted-by-receiver")]
    pub is_deleted_by_receiver: bool,
}

/// 返回给前端的消息模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_uuid: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub message_content: String,
    pub message_type: String,
    pub file_url: Option<String>,
    pub file_size: Option<i64>,
    pub send_time: DateTime<Utc>,
}

impl From<Message> for MessageResponse {
    fn from(msg: Message) -> Self {
        Self {
            message_uuid: msg.message_uuid,
            sender_id: msg.sender_id,
            receiver_id: msg.receiver_id,
            message_content: msg.message_content,
            message_type: msg.message_type,
            file_url: msg.file_url,
            file_size: msg.file_size,
            send_time: msg.send_time,
        }
    }
}

