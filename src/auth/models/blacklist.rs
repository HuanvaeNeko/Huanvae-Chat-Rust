use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Token 黑名单数据库模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlacklistToken {
    #[serde(rename = "jti")]
    #[sqlx(rename = "jti")]
    pub jti: String,

    #[serde(rename = "user-id")]
    #[sqlx(rename = "user-id")]
    pub user_id: String,

    #[serde(rename = "token-type")]
    #[sqlx(rename = "token-type")]
    pub token_type: String,

    #[serde(rename = "expires-at")]
    #[sqlx(rename = "expires-at")]
    pub expires_at: NaiveDateTime,

    #[serde(rename = "blacklisted-at")]
    #[sqlx(rename = "blacklisted-at")]
    pub blacklisted_at: NaiveDateTime,

    #[sqlx(rename = "reason")]
    pub reason: Option<String>,
}

