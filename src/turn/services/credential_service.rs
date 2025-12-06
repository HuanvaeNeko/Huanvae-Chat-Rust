//! 凭证签发服务
//!
//! 生成 TURN REST API 临时凭证

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::sync::Arc;

use crate::turn::models::protocol::{IceServer, IceServersResponse};
use crate::turn::models::SelectedNode;

use super::SecretManager;

type HmacSha1 = Hmac<Sha1>;

/// 凭证服务
pub struct CredentialService {
    /// 密钥管理器
    secret_manager: Arc<SecretManager>,
    /// 凭证有效期（秒）
    credential_ttl_secs: u64,
    /// 域名
    realm: String,
}

impl CredentialService {
    /// 创建凭证服务
    pub fn new(
        secret_manager: Arc<SecretManager>,
        credential_ttl_secs: u64,
        realm: String,
    ) -> Self {
        Self {
            secret_manager,
            credential_ttl_secs,
            realm,
        }
    }

    /// 生成 ICE 服务器配置
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `selected_nodes` - 选中的 TURN 节点
    pub async fn generate_ice_servers(
        &self,
        user_id: &str,
        selected_nodes: Vec<SelectedNode>,
    ) -> IceServersResponse {
        let secret = self.secret_manager.get_current_secret().await;
        let expires_at = Utc::now() + Duration::seconds(self.credential_ttl_secs as i64);
        let timestamp = expires_at.timestamp();

        // 生成用户名: timestamp:user_id
        let username = format!("{}:{}", timestamp, user_id);

        // 生成密码: base64(hmac_sha1(secret, username))
        let credential = self.hmac_sha1(&secret, &username);

        let mut ice_servers = vec![];

        // TURN 服务器（带认证）
        for node in selected_nodes {
            ice_servers.push(IceServer {
                urls: node.urls,
                username: Some(username.clone()),
                credential: Some(credential.clone()),
                credential_type: Some("password".to_string()),
            });
        }

        // STUN 服务器（公共，无需认证）
        ice_servers.push(IceServer {
            urls: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            username: None,
            credential: None,
            credential_type: None,
        });

        IceServersResponse {
            ice_servers,
            expires_at,
        }
    }

    /// 计算 HMAC-SHA1
    fn hmac_sha1(&self, secret: &str, message: &str) -> String {
        let mut mac =
            HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        BASE64.encode(mac.finalize().into_bytes())
    }

    /// 获取域名
    pub fn get_realm(&self) -> &str {
        &self.realm
    }

    /// 获取凭证有效期
    pub fn get_credential_ttl(&self) -> u64 {
        self.credential_ttl_secs
    }
}

