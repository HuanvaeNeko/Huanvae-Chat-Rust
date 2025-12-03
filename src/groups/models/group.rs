//! 群聊模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 入群模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JoinMode {
    /// 开放入群：所有方式均可直接进入
    Open,
    /// 需审核：普通成员邀请和搜索入群需审核
    ApprovalRequired,
    /// 仅邀请：只能通过邀请进入
    InviteOnly,
    /// 仅管理邀请：只能群主/管理员邀请进入
    AdminInviteOnly,
    /// 关闭入群：不允许任何新成员加入
    Closed,
}

impl JoinMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            JoinMode::Open => "open",
            JoinMode::ApprovalRequired => "approval_required",
            JoinMode::InviteOnly => "invite_only",
            JoinMode::AdminInviteOnly => "admin_invite_only",
            JoinMode::Closed => "closed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(JoinMode::Open),
            "approval_required" => Some(JoinMode::ApprovalRequired),
            "invite_only" => Some(JoinMode::InviteOnly),
            "admin_invite_only" => Some(JoinMode::AdminInviteOnly),
            "closed" => Some(JoinMode::Closed),
            _ => None,
        }
    }
}

impl Default for JoinMode {
    fn default() -> Self {
        JoinMode::ApprovalRequired
    }
}

/// 群聊状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    Active,
    Disbanded,
}

impl GroupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupStatus::Active => "active",
            GroupStatus::Disbanded => "disbanded",
        }
    }
}

/// 群聊数据库模型
#[derive(Debug, Clone, FromRow)]
pub struct Group {
    #[sqlx(rename = "group-id")]
    pub group_id: Uuid,
    #[sqlx(rename = "group-name")]
    pub group_name: String,
    #[sqlx(rename = "group-avatar-url")]
    pub group_avatar_url: Option<String>,
    #[sqlx(rename = "group-description")]
    pub group_description: Option<String>,
    #[sqlx(rename = "creator-id")]
    pub creator_id: String,
    #[sqlx(rename = "created-at")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "join-mode")]
    pub join_mode: String,
    pub status: String,
    #[sqlx(rename = "disbanded-at")]
    pub disbanded_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "disbanded-by")]
    pub disbanded_by: Option<String>,
    #[sqlx(rename = "member-count")]
    pub member_count: i32,
    #[sqlx(rename = "updated-at")]
    pub updated_at: DateTime<Utc>,
}

/// 群聊信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub group_id: String,
    pub group_name: String,
    pub group_avatar_url: Option<String>,
    pub group_description: Option<String>,
    pub creator_id: String,
    pub created_at: String,
    pub join_mode: String,
    pub status: String,
    pub member_count: i32,
}

impl From<Group> for GroupInfo {
    fn from(g: Group) -> Self {
        Self {
            group_id: g.group_id.to_string(),
            group_name: g.group_name,
            group_avatar_url: g.group_avatar_url,
            group_description: g.group_description,
            creator_id: g.creator_id,
            created_at: g.created_at.to_rfc3339(),
            join_mode: g.join_mode,
            status: g.status,
            member_count: g.member_count,
        }
    }
}

