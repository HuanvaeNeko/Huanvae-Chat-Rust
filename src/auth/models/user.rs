use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 用户模型（对应数据库 users 表）
/// 好友关系已移至独立表: friendships, friend_requests
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    #[serde(rename = "user-id")]
    #[sqlx(rename = "user-id")]
    pub user_id: String,

    #[serde(rename = "user-nickname")]
    #[sqlx(rename = "user-nickname")]
    pub user_nickname: String,

    #[serde(rename = "user-password")]
    #[sqlx(rename = "user-password")]
    pub user_password: String,

    #[serde(rename = "user-email")]
    #[sqlx(rename = "user-email")]
    pub user_email: String,

    #[sqlx(rename = "admin")]
    pub admin: String,

    #[serde(rename = "need-blacklist-check")]
    #[sqlx(rename = "need-blacklist-check")]
    pub need_blacklist_check: bool,

    #[serde(rename = "blacklist-check-expires-at")]
    #[sqlx(rename = "blacklist-check-expires-at")]
    pub blacklist_check_expires_at: Option<NaiveDateTime>,

    #[serde(rename = "created-at")]
    #[sqlx(rename = "created-at")]
    pub created_at: NaiveDateTime,

    #[serde(rename = "updated-at")]
    #[sqlx(rename = "updated-at")]
    pub updated_at: NaiveDateTime,
}

/// 用户注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub user_id: String,  // 用户提供的登录ID
    pub nickname: String,
    pub email: Option<String>,
    pub password: String,
}

/// 用户登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub user_id: String,
    pub password: String,
    pub device_info: Option<String>,
    pub mac_address: Option<String>,
}

/// 用户信息响应（不包含密码）
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: String,
    pub nickname: String,
    pub email: String,
    pub admin: bool,
    pub created_at: NaiveDateTime,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            user_id: user.user_id,
            nickname: user.user_nickname,
            email: user.user_email,
            admin: user.admin == "true",
            created_at: user.created_at,
        }
    }
}

