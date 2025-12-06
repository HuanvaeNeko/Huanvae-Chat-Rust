//! 参与者数据模型

use axum::extract::ws::Message;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::mpsc;

/// 参与者信息
#[derive(Debug, Clone, Serialize)]
pub struct Participant {
    /// 参与者 ID
    pub participant_id: String,
    /// 显示名称
    pub display_name: String,
    /// 关联的用户 ID（创建者有，访客为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 是否是创建者
    pub is_creator: bool,
    /// 加入时间
    pub joined_at: DateTime<Utc>,
}

/// 参与者简要信息（用于列表展示）
#[derive(Debug, Clone, Serialize)]
pub struct ParticipantInfo {
    /// 参与者 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 是否是创建者
    pub is_creator: bool,
}

impl From<&Participant> for ParticipantInfo {
    fn from(p: &Participant) -> Self {
        Self {
            id: p.participant_id.clone(),
            name: p.display_name.clone(),
            is_creator: p.is_creator,
        }
    }
}

/// 参与者连接信息（内部使用）
#[derive(Debug)]
pub struct ParticipantConnection {
    /// 参与者信息
    pub participant: Participant,
    /// 消息发送通道
    pub sender: mpsc::UnboundedSender<Message>,
}

