//! 通信协议定义
//!
//! 定义 Server ↔ Agent 之间的消息格式

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::node::{NodeCapabilities, NodeMetrics, TurnPorts};

// ============================================
// Agent -> Server 消息
// ============================================

/// Agent 发送给服务器的消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// 注册请求
    Register {
        node_id: String,
        region: String,
        public_ip: String,
        ports: TurnPorts,
        capabilities: NodeCapabilities,
    },

    /// 心跳（包含指标）
    Heartbeat { metrics: NodeMetrics },

    /// 配置应用确认
    ConfigApplied {
        config_version: u64,
        success: bool,
        #[serde(default)]
        error: Option<String>,
    },

    /// 请求重新下发配置
    RequestConfig,
}

// ============================================
// Server -> Agent 消息
// ============================================

/// 服务器发送给 Agent 的消息
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoordinatorMessage {
    /// 注册确认
    Registered {
        node_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        assigned_id: Option<String>,
    },

    /// 下发配置
    Config { version: u64, config: TurnConfig },

    /// 更新密钥
    UpdateSecret {
        secret: String,
        effective_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    },

    /// 控制命令
    Command { command: NodeCommand },

    /// 错误
    Error { code: String, message: String },
}

/// TURN 配置（下发给 Agent）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnConfig {
    /// 域名
    pub realm: String,
    /// 认证密钥
    pub auth_secret: String,
    /// 总配额
    pub total_quota: u32,
    /// 单用户配额
    pub user_quota: u32,
    /// 最大带宽 (bps)
    pub max_bps: u64,
}

/// 节点控制命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCommand {
    /// 重载配置
    Reload,
    /// 立即关闭
    Shutdown,
    /// 排空后关闭
    DrainAndShutdown,
}

impl CoordinatorMessage {
    /// 序列化为 JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl AgentMessage {
    /// 从 JSON 反序列化
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ============================================
// 客户端 API 响应
// ============================================

/// ICE 服务器配置
#[derive(Debug, Clone, Serialize)]
pub struct IceServer {
    /// TURN/STUN URL 列表
    pub urls: Vec<String>,
    /// 用户名（TURN 需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 凭证（TURN 需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
    /// 凭证类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_type: Option<String>,
}

/// ICE 服务器响应
#[derive(Debug, Clone, Serialize)]
pub struct IceServersResponse {
    /// ICE 服务器列表
    pub ice_servers: Vec<IceServer>,
    /// 凭证过期时间
    pub expires_at: DateTime<Utc>,
}

