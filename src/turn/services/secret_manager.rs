//! 密钥管理服务
//!
//! 管理 TURN 认证密钥，支持自动轮换

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::info;

use super::NodeRegistry;
use crate::turn::models::protocol::CoordinatorMessage;

/// 密钥条目
#[derive(Clone)]
pub struct SecretEntry {
    /// 密钥值
    pub secret: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 密钥管理器
pub struct SecretManager {
    /// 当前密钥
    current_secret: RwLock<SecretEntry>,
    /// 上一个密钥（用于过渡期）
    previous_secret: RwLock<Option<SecretEntry>>,
    /// 轮换间隔（小时）
    rotation_hours: u64,
}

impl SecretManager {
    /// 创建密钥管理器
    pub fn new(rotation_hours: u64) -> Self {
        let secret = Self::generate_secret();
        let now = Utc::now();

        Self {
            current_secret: RwLock::new(SecretEntry {
                secret,
                created_at: now,
                // 密钥有效期为轮换间隔的 2 倍（确保过渡期）
                expires_at: now + Duration::hours(rotation_hours as i64 * 2),
            }),
            previous_secret: RwLock::new(None),
            rotation_hours,
        }
    }

    /// 生成随机密钥
    fn generate_secret() -> String {
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect()
    }

    /// 获取当前密钥
    pub async fn get_current_secret(&self) -> String {
        self.current_secret.read().await.secret.clone()
    }

    /// 获取当前密钥条目
    pub async fn get_current_entry(&self) -> SecretEntry {
        self.current_secret.read().await.clone()
    }

    /// 轮换密钥
    pub async fn rotate(&self) -> SecretEntry {
        let new_secret = Self::generate_secret();
        let now = Utc::now();

        let new_entry = SecretEntry {
            secret: new_secret,
            created_at: now,
            expires_at: now + Duration::hours(self.rotation_hours as i64 * 2),
        };

        // 保存旧密钥（用于过渡期验证）
        let old = self.current_secret.read().await.clone();
        *self.previous_secret.write().await = Some(old);

        // 更新新密钥
        *self.current_secret.write().await = new_entry.clone();

        info!("密钥已轮换");

        new_entry
    }

    /// 启动自动轮换任务
    pub fn start_rotation_task(self: Arc<Self>, registry: Arc<NodeRegistry>) {
        let rotation_interval = TokioDuration::from_secs(self.rotation_hours * 3600);

        tokio::spawn(async move {
            let mut ticker = interval(rotation_interval);

            // 跳过第一次立即触发
            ticker.tick().await;

            loop {
                ticker.tick().await;

                let new_entry = self.rotate().await;

                // 通知所有节点更新密钥
                let msg = CoordinatorMessage::UpdateSecret {
                    secret: new_entry.secret,
                    effective_at: new_entry.created_at,
                    expires_at: new_entry.expires_at,
                };

                registry.broadcast(&msg);

                info!(
                    "密钥轮换完成，已通知 {} 个节点",
                    registry.node_count()
                );
            }
        });
    }
}

