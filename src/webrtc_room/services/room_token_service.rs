//! 房间临时 Token 服务
//!
//! 生成和验证用于 WebSocket 连接的临时 Token

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64, Engine};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 房间 Token 声明
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTokenClaims {
    /// 参与者 ID
    pub participant_id: String,
    /// 房间 ID
    pub room_id: String,
    /// 显示名称
    pub display_name: String,
    /// 是否是创建者
    pub is_creator: bool,
    /// 关联的用户 ID（创建者有）
    pub user_id: Option<String>,
    /// 过期时间戳
    pub exp: i64,
}

/// 房间 Token 服务
pub struct RoomTokenService {
    /// 签名密钥
    secret: String,
    /// Token 有效期（秒）
    ttl_secs: u64,
}

impl RoomTokenService {
    /// 创建 Token 服务
    pub fn new(secret: String, ttl_secs: u64) -> Self {
        Self { secret, ttl_secs }
    }

    /// 生成 Token
    pub fn generate_token(&self, claims: &RoomTokenClaims) -> String {
        // 序列化 claims
        let payload = serde_json::to_string(claims).unwrap_or_default();
        let payload_b64 = BASE64.encode(payload.as_bytes());

        // 计算签名
        let signature = self.sign(&payload_b64);

        // 组合: payload.signature
        format!("{}.{}", payload_b64, signature)
    }

    /// 验证并解析 Token
    pub fn verify_token(&self, token: &str) -> Result<RoomTokenClaims, TokenError> {
        // 分割 token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Err(TokenError::InvalidFormat);
        }

        let payload_b64 = parts[0];
        let signature = parts[1];

        // 验证签名
        let expected_signature = self.sign(payload_b64);
        if signature != expected_signature {
            return Err(TokenError::InvalidSignature);
        }

        // 解码 payload
        let payload_bytes = BASE64
            .decode(payload_b64)
            .map_err(|_| TokenError::InvalidFormat)?;

        let payload_str =
            String::from_utf8(payload_bytes).map_err(|_| TokenError::InvalidFormat)?;

        // 解析 claims
        let claims: RoomTokenClaims =
            serde_json::from_str(&payload_str).map_err(|_| TokenError::InvalidFormat)?;

        // 检查过期
        if claims.exp < Utc::now().timestamp() {
            return Err(TokenError::Expired);
        }

        Ok(claims)
    }

    /// 创建 Token Claims
    pub fn create_claims(
        &self,
        participant_id: String,
        room_id: String,
        display_name: String,
        is_creator: bool,
        user_id: Option<String>,
    ) -> RoomTokenClaims {
        let exp = (Utc::now() + Duration::seconds(self.ttl_secs as i64)).timestamp();

        RoomTokenClaims {
            participant_id,
            room_id,
            display_name,
            is_creator,
            user_id,
            exp,
        }
    }

    /// 获取过期时间
    pub fn get_expires_at(&self) -> chrono::DateTime<Utc> {
        Utc::now() + Duration::seconds(self.ttl_secs as i64)
    }

    /// 计算签名
    fn sign(&self, data: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        BASE64.encode(mac.finalize().into_bytes())
    }
}

/// Token 错误
#[derive(Debug, Clone)]
pub enum TokenError {
    /// 格式无效
    InvalidFormat,
    /// 签名无效
    InvalidSignature,
    /// 已过期
    Expired,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::InvalidFormat => write!(f, "Token 格式无效"),
            TokenError::InvalidSignature => write!(f, "Token 签名无效"),
            TokenError::Expired => write!(f, "Token 已过期"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generate_and_verify() {
        let service = RoomTokenService::new("test-secret".to_string(), 600);

        let claims = service.create_claims(
            "p123".to_string(),
            "ABC123".to_string(),
            "测试用户".to_string(),
            false,
            None,
        );

        let token = service.generate_token(&claims);
        let verified = service.verify_token(&token).unwrap();

        assert_eq!(verified.participant_id, "p123");
        assert_eq!(verified.room_id, "ABC123");
        assert_eq!(verified.display_name, "测试用户");
        assert!(!verified.is_creator);
    }
}

