//! 节点数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// 节点状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// 注册中
    Registering,
    /// 活跃
    Active,
    /// 不健康（心跳超时）
    Unhealthy,
    /// 排空中
    Draining,
    /// 离线
    Offline,
}

/// 节点完整状态
#[derive(Debug, Clone)]
pub struct NodeState {
    /// 节点 ID
    pub node_id: String,
    /// 区域
    pub region: String,
    /// 公网 IP
    pub public_ip: String,
    /// 端口配置
    pub ports: TurnPorts,
    /// 节点能力
    pub capabilities: NodeCapabilities,
    /// 运行指标
    pub metrics: NodeMetrics,
    /// 节点状态
    pub status: NodeStatus,
    /// 注册时间
    pub registered_at: DateTime<Utc>,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 当前配置版本
    pub config_version: u64,
}

impl NodeState {
    /// 构建 TURN URL 列表
    pub fn build_turn_urls(&self) -> Vec<String> {
        let mut urls = vec![];

        // UDP
        urls.push(format!("turn:{}:{}", self.public_ip, self.ports.listening));

        // TCP
        if self.capabilities.supports_tcp {
            urls.push(format!(
                "turn:{}:{}?transport=tcp",
                self.public_ip, self.ports.listening
            ));
        }

        // TLS
        if self.capabilities.supports_tls {
            urls.push(format!("turns:{}:{}", self.public_ip, self.ports.tls));
        }

        urls
    }
}

/// 选中的节点（用于返回给客户端）
#[derive(Debug, Clone, Serialize)]
pub struct SelectedNode {
    /// 节点 ID
    pub node_id: String,
    /// 区域
    pub region: String,
    /// TURN URL 列表
    pub urls: Vec<String>,
    /// 评分
    pub score: f64,
}

