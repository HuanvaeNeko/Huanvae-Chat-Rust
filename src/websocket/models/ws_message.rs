//! WebSocket 消息协议定义
//!
//! 定义服务端和客户端之间的消息格式

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ========================================
// 服务端 → 客户端消息
// ========================================

/// 服务端发送的消息类型
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// 连接成功响应
    Connected {
        /// 未读消息摘要
        unread_summary: UnreadSummary,
    },

    /// 新消息通知
    NewMessage {
        /// 来源类型：friend 或 group
        source_type: SourceType,
        /// 来源 ID（好友 ID 或群 ID）
        source_id: String,
        /// 消息 UUID
        message_uuid: String,
        /// 发送者 ID
        sender_id: String,
        /// 发送者昵称
        sender_nickname: String,
        /// 消息预览（截断）
        preview: String,
        /// 消息类型
        message_type: String,
        /// 发送时间
        timestamp: DateTime<Utc>,
    },

    /// 消息撤回通知
    MessageRecalled {
        /// 来源类型
        source_type: SourceType,
        /// 来源 ID
        source_id: String,
        /// 消息 UUID
        message_uuid: String,
        /// 撤回者 ID
        recalled_by: String,
    },

    /// 已读同步通知（当对方已读时通知发送方）
    ReadSync {
        /// 来源类型
        source_type: SourceType,
        /// 来源 ID
        source_id: String,
        /// 已读用户 ID
        reader_id: String,
        /// 已读时间
        read_at: DateTime<Utc>,
    },

    /// 系统通知
    SystemNotification {
        /// 通知类型
        notification_type: SystemNotificationType,
        /// 通知数据
        data: serde_json::Value,
    },

    /// 心跳响应
    Pong {
        /// 服务器时间戳
        timestamp: DateTime<Utc>,
    },

    /// 错误消息
    Error {
        /// 错误码
        code: String,
        /// 错误描述
        message: String,
    },
}

/// 来源类型枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// 好友私信
    Friend,
    /// 群聊
    Group,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Friend => write!(f, "friend"),
            SourceType::Group => write!(f, "group"),
        }
    }
}

/// 系统通知类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemNotificationType {
    /// 新好友申请
    FriendRequest,
    /// 好友申请已通过
    FriendRequestApproved,
    /// 好友申请被拒绝
    FriendRequestRejected,
    /// 群邀请
    GroupInvite,
    /// 入群申请
    GroupJoinRequest,
    /// 入群申请已通过
    GroupJoinApproved,
    /// 被移出群聊
    GroupRemoved,
    /// 群解散
    GroupDisbanded,
    /// 群公告更新
    GroupNoticeUpdated,
}

// ========================================
// 客户端 → 服务端消息
// ========================================

/// 客户端发送的消息类型
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// 标记已读
    MarkRead {
        /// 目标类型：friend 或 group
        target_type: SourceType,
        /// 目标 ID（好友 ID 或群 ID）
        target_id: String,
    },

    /// 心跳
    Ping,

    /// 订阅在线状态（预留）
    SubscribePresence {
        /// 要订阅的用户 ID 列表
        user_ids: Vec<String>,
    },
}

// ========================================
// 未读消息摘要
// ========================================

/// 未读消息总摘要
#[derive(Debug, Clone, Serialize, Default)]
pub struct UnreadSummary {
    /// 好友未读列表
    pub friend_unreads: Vec<FriendUnread>,
    /// 群聊未读列表
    pub group_unreads: Vec<GroupUnread>,
    /// 总未读数
    pub total_count: i32,
}

/// 好友未读消息摘要
#[derive(Debug, Clone, Serialize)]
pub struct FriendUnread {
    /// 好友 ID
    pub friend_id: String,
    /// 好友昵称
    pub friend_nickname: String,
    /// 好友头像
    pub friend_avatar: String,
    /// 未读数量
    pub unread_count: i32,
    /// 最后一条消息预览
    pub last_message_preview: String,
    /// 最后一条消息类型
    pub last_message_type: String,
    /// 最后消息时间
    pub last_message_time: Option<DateTime<Utc>>,
}

/// 群聊未读消息摘要
#[derive(Debug, Clone, Serialize)]
pub struct GroupUnread {
    /// 群 ID
    pub group_id: String,
    /// 群名称
    pub group_name: String,
    /// 群头像
    pub group_avatar: String,
    /// 未读数量
    pub unread_count: i32,
    /// 最后一条消息预览
    pub last_message_preview: String,
    /// 最后一条消息类型
    pub last_message_type: String,
    /// 最后发送者昵称
    pub last_sender_nickname: String,
    /// 最后消息时间
    pub last_message_time: Option<DateTime<Utc>>,
}

// ========================================
// 辅助方法
// ========================================

impl ServerMessage {
    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error","code":"serialize_error","message":"Failed to serialize message"}"#.to_string())
    }
}

impl ClientMessage {
    /// 从 JSON 字符串解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// 截断消息内容作为预览
pub fn truncate_preview(content: &str, max_len: usize) -> String {
    if content.chars().count() <= max_len {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

