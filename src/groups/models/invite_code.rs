//! 邀请码模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 邀请码类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InviteCodeType {
    Direct,  // 直通码：群主/管理员生成，可直接入群
    Normal,  // 普通码：普通成员生成，需审核
}

impl InviteCodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InviteCodeType::Direct => "direct",
            InviteCodeType::Normal => "normal",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "direct" => Some(InviteCodeType::Direct),
            "normal" => Some(InviteCodeType::Normal),
            _ => None,
        }
    }
}

/// 邀请码状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InviteCodeStatus {
    Active,
    Expired,
    Revoked,
    Exhausted,
}

impl InviteCodeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InviteCodeStatus::Active => "active",
            InviteCodeStatus::Expired => "expired",
            InviteCodeStatus::Revoked => "revoked",
            InviteCodeStatus::Exhausted => "exhausted",
        }
    }
}

/// 邀请码数据库模型
#[derive(Debug, Clone, FromRow)]
pub struct InviteCode {
    pub id: Uuid,
    #[sqlx(rename = "group-id")]
    pub group_id: Uuid,
    pub code: String,
    #[sqlx(rename = "code-type")]
    pub code_type: String,
    #[sqlx(rename = "creator-id")]
    pub creator_id: String,
    #[sqlx(rename = "creator-role")]
    pub creator_role: String,
    #[sqlx(rename = "max-uses")]
    pub max_uses: Option<i32>,
    #[sqlx(rename = "used-count")]
    pub used_count: i32,
    #[sqlx(rename = "expires-at")]
    pub expires_at: Option<DateTime<Utc>>,
    pub status: String,
    #[sqlx(rename = "revoked-at")]
    pub revoked_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "revoked-by")]
    pub revoked_by: Option<String>,
    #[sqlx(rename = "created-at")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updated-at")]
    pub updated_at: DateTime<Utc>,
}

/// 邀请码信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCodeInfo {
    pub id: String,
    pub code: String,
    pub code_type: String,
    pub creator_id: String,
    pub max_uses: Option<i32>,
    pub used_count: i32,
    pub expires_at: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<InviteCode> for InviteCodeInfo {
    fn from(ic: InviteCode) -> Self {
        Self {
            id: ic.id.to_string(),
            code: ic.code,
            code_type: ic.code_type,
            creator_id: ic.creator_id,
            max_uses: ic.max_uses,
            used_count: ic.used_count,
            expires_at: ic.expires_at.map(|t| t.to_rfc3339()),
            status: ic.status,
            created_at: ic.created_at.to_rfc3339(),
        }
    }
}

/// 邀请码列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InviteCodeListResponse {
    pub codes: Vec<InviteCodeInfo>,
}

/// 创建邀请码响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInviteCodeResponse {
    pub id: String,
    pub code: String,
    pub code_type: String,
    pub expires_at: Option<String>,
}

