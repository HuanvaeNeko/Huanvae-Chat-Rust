//! 群成员模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 群成员角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

impl MemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(MemberRole::Owner),
            "admin" => Some(MemberRole::Admin),
            "member" => Some(MemberRole::Member),
            _ => None,
        }
    }
}

/// 成员状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Removed,
    Left,
}

impl MemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberStatus::Active => "active",
            MemberStatus::Removed => "removed",
            MemberStatus::Left => "left",
        }
    }
}

/// 入群方式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinMethod {
    Create,
    OwnerInvite,
    AdminInvite,
    MemberInvite,
    DirectInviteCode,
    NormalInviteCode,
    SearchDirect,
    SearchApproved,
}

impl JoinMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            JoinMethod::Create => "create",
            JoinMethod::OwnerInvite => "owner_invite",
            JoinMethod::AdminInvite => "admin_invite",
            JoinMethod::MemberInvite => "member_invite",
            JoinMethod::DirectInviteCode => "direct_invite_code",
            JoinMethod::NormalInviteCode => "normal_invite_code",
            JoinMethod::SearchDirect => "search_direct",
            JoinMethod::SearchApproved => "search_approved",
        }
    }
}

/// 群成员数据库模型
#[derive(Debug, Clone, FromRow)]
pub struct GroupMember {
    pub id: Uuid,
    #[sqlx(rename = "group-id")]
    pub group_id: Uuid,
    #[sqlx(rename = "user-id")]
    pub user_id: String,
    pub role: String,
    #[sqlx(rename = "group-nickname")]
    pub group_nickname: Option<String>,
    #[sqlx(rename = "joined-at")]
    pub joined_at: DateTime<Utc>,
    #[sqlx(rename = "join-method")]
    pub join_method: String,
    #[sqlx(rename = "invited-by")]
    pub invited_by: Option<String>,
    #[sqlx(rename = "approved-by")]
    pub approved_by: Option<String>,
    #[sqlx(rename = "invite-code-id")]
    pub invite_code_id: Option<Uuid>,
    pub status: String,
    #[sqlx(rename = "left-at")]
    pub left_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "left-reason")]
    pub left_reason: Option<String>,
    #[sqlx(rename = "removed-by")]
    pub removed_by: Option<String>,
    #[sqlx(rename = "removed-reason")]
    pub removed_reason: Option<String>,
    #[sqlx(rename = "muted-until")]
    pub muted_until: Option<DateTime<Utc>>,
    #[sqlx(rename = "muted-by")]
    pub muted_by: Option<String>,
    #[sqlx(rename = "muted-reason")]
    pub muted_reason: Option<String>,
    #[sqlx(rename = "updated-at")]
    pub updated_at: DateTime<Utc>,
}

/// 群成员信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub user_id: String,
    pub user_nickname: Option<String>,
    pub user_avatar_url: Option<String>,
    pub role: String,
    pub group_nickname: Option<String>,
    pub joined_at: String,
    pub join_method: String,
    pub muted_until: Option<String>,
}

/// 用户在群中的简要信息
#[derive(Debug, Clone, FromRow)]
pub struct MemberBrief {
    #[sqlx(rename = "user-id")]
    pub user_id: String,
    pub role: String,
    pub status: String,
    #[sqlx(rename = "muted-until")]
    pub muted_until: Option<DateTime<Utc>>,
}

/// 权限要求枚举
/// 
/// 用于统一的权限验证方法 `check_permission`
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequiredPermission {
    /// 任何活跃成员
    ActiveMember,
    /// 管理员或群主
    AdminOrOwner,
    /// 仅群主
    OwnerOnly,
}

