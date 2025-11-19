use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Refresh Token 数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    #[serde(rename = "token-id")]
    #[sqlx(rename = "token-id")]
    pub token_id: String,

    #[serde(rename = "user-id")]
    #[sqlx(rename = "user-id")]
    pub user_id: String,

    #[serde(rename = "refresh-token")]
    #[sqlx(rename = "refresh-token")]
    pub refresh_token: String,

    #[serde(rename = "device-id")]
    #[sqlx(rename = "device-id")]
    pub device_id: String,

    #[serde(rename = "device-info")]
    #[sqlx(rename = "device-info")]
    pub device_info: Option<String>,

    #[serde(rename = "ip-address")]
    #[sqlx(rename = "ip-address")]
    pub ip_address: Option<String>,

    #[serde(rename = "created-at")]
    #[sqlx(rename = "created-at")]
    pub created_at: NaiveDateTime,

    #[serde(rename = "expires-at")]
    #[sqlx(rename = "expires-at")]
    pub expires_at: NaiveDateTime,

    #[serde(rename = "last-used-at")]
    #[sqlx(rename = "last-used-at")]
    pub last_used_at: Option<NaiveDateTime>,

    #[serde(rename = "is-revoked")]
    #[sqlx(rename = "is-revoked")]
    pub is_revoked: bool,

    #[serde(rename = "revoked-at")]
    #[sqlx(rename = "revoked-at")]
    pub revoked_at: Option<NaiveDateTime>,

    #[serde(rename = "revoked-reason")]
    #[sqlx(rename = "revoked-reason")]
    pub revoked_reason: Option<String>,
}

/// 创建 Refresh Token 的参数
#[derive(Debug)]
pub struct CreateRefreshToken {
    pub token_id: String,
    pub user_id: String,
    pub refresh_token: String,
    pub device_id: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: NaiveDateTime,
}

