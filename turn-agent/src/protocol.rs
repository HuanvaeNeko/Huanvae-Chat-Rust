//! 通信协议定义
//!
//! 定义 Agent ↔ Server 之间的消息格式

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================
// Agent -> Server 消息
// ============================================

/// Agent 发送给服务器的消息
#[derive(Debug, Clone, Serialize)]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// 请求重新下发配置
    #[allow(dead_code)]
    RequestConfig,
}

/// TURN 端口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnPorts {
    /// 监听端口 (UDP/TCP)
    pub listening: u16,
    /// TLS 端口
    pub tls: u16,
    /// 中继最小端口
    pub min_relay: u16,
    /// 中继最大端口
    pub max_relay: u16,
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// 支持 TCP
    pub supports_tcp: bool,
    /// 支持 TLS
    pub supports_tls: bool,
    /// 支持 DTLS
    pub supports_dtls: bool,
    /// 最大带宽 (Mbps)
    pub max_bandwidth_mbps: u32,
}

/// 节点运行指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// 活跃会话数
    pub active_sessions: u32,
    /// 总分配次数
    pub total_allocations: u64,
    /// 入站带宽 (Mbps)
    pub bandwidth_in_mbps: f64,
    /// 出站带宽 (Mbps)
    pub bandwidth_out_mbps: f64,
    /// CPU 使用率 (%)
    pub cpu_percent: f32,
    /// 内存使用率 (%)
    pub memory_percent: f32,
    /// 运行时间 (秒)
    pub uptime_seconds: u64,
}

// ============================================
// Server -> Agent 消息
// ============================================

/// 服务器发送给 Agent 的消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoordinatorMessage {
    /// 注册确认
    Registered {
        node_id: String,
        #[serde(default)]
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

/// TURN 配置（由服务器下发）
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCommand {
    /// 重载配置
    Reload,
    /// 立即关闭
    Shutdown,
    /// 排空后关闭
    DrainAndShutdown,
}

impl AgentMessage {
    /// 序列化为 JSON
    #[allow(dead_code)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl CoordinatorMessage {
    /// 从 JSON 反序列化
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

