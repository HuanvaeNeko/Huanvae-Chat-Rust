use serde::{Deserialize, Serialize};

/// Access Token 的 JWT Claims（15分钟有效）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// 用户ID (subject)
    pub sub: String,
    
    /// 用户邮箱
    pub email: String,
    
    /// 设备ID
    pub device_id: String,
    
    /// 设备信息（操作系统、浏览器等）
    pub device_info: String,
    
    /// MAC地址
    pub mac_address: String,
    
    /// JWT ID（用于黑名单）
    pub jti: String,
    
    /// 过期时间（Unix时间戳）
    pub exp: i64,
    
    /// 签发时间（Unix时间戳）
    pub iat: i64,
}

/// Refresh Token 的 JWT Claims（7天有效）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// 用户ID (subject)
    pub sub: String,
    
    /// 设备ID
    pub device_id: String,
    
    /// 数据库中的 token-id（用于查询和撤销）
    pub token_id: String,
    
    /// 过期时间（Unix时间戳）
    pub exp: i64,
    
    /// 签发时间（Unix时间戳）
    pub iat: i64,
}

/// Token 响应
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Access Token（15分钟）
    pub access_token: String,
    
    /// Refresh Token（7天）
    pub refresh_token: String,
    
    /// Token 类型
    pub token_type: String,
    
    /// Access Token 过期时间（秒）
    pub expires_in: i64,
}

/// 刷新 Token 请求
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

