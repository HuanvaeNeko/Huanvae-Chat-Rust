use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 用户完整信息响应（不含密码）
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileResponse {
    #[serde(rename = "user_id")]
    pub user_id: String,
    
    #[serde(rename = "user_nickname")]
    pub user_nickname: String,
    
    #[serde(rename = "user_email")]
    pub user_email: Option<String>,
    
    #[serde(rename = "user_signature")]
    pub user_signature: Option<String>,
    
    #[serde(rename = "user_avatar_url")]
    pub user_avatar_url: Option<String>,
    
    #[serde(rename = "admin")]
    pub admin: String,
    
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
}

/// 头像上传响应
#[derive(Debug, Serialize, Deserialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
    pub message: String,
}

