//! 配置加载模块
//!
//! 从 .env 文件加载 Agent 配置

use std::env;
use thiserror::Error;
use uuid::Uuid;

/// Agent 配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// 节点 ID
    pub node_id: String,
    /// 节点区域
    pub region: String,
    /// 公网 IP
    pub public_ip: String,
    /// 主服务器 WebSocket 地址
    pub coordinator_url: String,
    /// Agent 认证令牌
    pub coordinator_token: String,
    /// TURN 监听端口
    pub turn_port: u16,
    /// TURN TLS 端口
    pub turn_tls_port: u16,
    /// 中继最小端口
    pub relay_min_port: u16,
    /// 中继最大端口
    pub relay_max_port: u16,
    /// 心跳间隔（秒）
    pub heartbeat_interval: u64,
}

/// 配置错误
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("缺少必填配置: {0}")]
    Missing(&'static str),

    #[error("配置值无效: {0}")]
    Invalid(String),
}

impl AgentConfig {
    /// 从环境变量加载配置
    pub fn load() -> Result<Self, ConfigError> {
        // 加载 .env 文件
        dotenvy::dotenv().ok();

        // ========== 必填配置 ==========

        let public_ip = env::var("PUBLIC_IP")
            .map_err(|_| ConfigError::Missing("PUBLIC_IP"))?;

        // 验证 IP 格式
        if public_ip == "你的公网IP" || public_ip.is_empty() {
            return Err(ConfigError::Invalid(
                "PUBLIC_IP 未配置或使用了默认值".to_string(),
            ));
        }

        let coordinator_url = env::var("COORDINATOR_URL")
            .map_err(|_| ConfigError::Missing("COORDINATOR_URL"))?;

        if coordinator_url.contains("example.com") {
            return Err(ConfigError::Invalid(
                "COORDINATOR_URL 未配置或使用了默认值".to_string(),
            ));
        }

        let coordinator_token = env::var("COORDINATOR_TOKEN")
            .map_err(|_| ConfigError::Missing("COORDINATOR_TOKEN"))?;

        if coordinator_token == "your-agent-token-here" || coordinator_token.is_empty() {
            return Err(ConfigError::Invalid(
                "COORDINATOR_TOKEN 未配置或使用了默认值".to_string(),
            ));
        }

        // ========== 可选配置 ==========

        let node_id = env::var("NODE_ID")
            .unwrap_or_else(|_| format!("turn-{}", &Uuid::new_v4().to_string()[..8]));

        let region = env::var("REGION").unwrap_or_else(|_| "unknown".to_string());

        let turn_port = env::var("TURN_PORT")
            .unwrap_or_else(|_| "3478".to_string())
            .parse()
            .unwrap_or(3478);

        let turn_tls_port = env::var("TURN_TLS_PORT")
            .unwrap_or_else(|_| "5349".to_string())
            .parse()
            .unwrap_or(5349);

        let relay_min_port = env::var("RELAY_MIN_PORT")
            .unwrap_or_else(|_| "49152".to_string())
            .parse()
            .unwrap_or(49152);

        let relay_max_port = env::var("RELAY_MAX_PORT")
            .unwrap_or_else(|_| "65535".to_string())
            .parse()
            .unwrap_or(65535);

        let heartbeat_interval = env::var("HEARTBEAT_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        Ok(Self {
            node_id,
            region,
            public_ip,
            coordinator_url,
            coordinator_token,
            turn_port,
            turn_tls_port,
            relay_min_port,
            relay_max_port,
            heartbeat_interval,
        })
    }
}

