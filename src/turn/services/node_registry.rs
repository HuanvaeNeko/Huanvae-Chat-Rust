//! 节点注册管理服务
//!
//! 管理已注册的 TURN 节点

use axum::extract::ws::Message;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::turn::models::{
    NodeCapabilities, NodeMetrics, NodeState, NodeStatus, TurnPorts,
};
use crate::turn::models::protocol::{CoordinatorMessage, TurnConfig};

/// 节点注册中心
pub struct NodeRegistry {
    /// 已注册节点: node_id -> NodeState
    nodes: DashMap<String, NodeState>,
    /// 节点消息发送通道: node_id -> Sender
    senders: DashMap<String, mpsc::UnboundedSender<Message>>,
    /// 当前配置版本
    config_version: AtomicU64,
    /// 心跳超时时间（秒）
    heartbeat_timeout_secs: u64,
}

impl NodeRegistry {
    /// 创建节点注册中心
    pub fn new(heartbeat_timeout_secs: u64) -> Self {
        Self {
            nodes: DashMap::new(),
            senders: DashMap::new(),
            config_version: AtomicU64::new(1),
            heartbeat_timeout_secs,
        }
    }

    /// 注册节点
    pub fn register(
        &self,
        node_id: String,
        region: String,
        public_ip: String,
        ports: TurnPorts,
        capabilities: NodeCapabilities,
        sender: mpsc::UnboundedSender<Message>,
    ) -> String {
        // 检查 ID 是否冲突
        let final_id = if self.nodes.contains_key(&node_id) {
            // 如果已存在，可能是重连，更新连接
            info!("节点 {} 重新连接", node_id);
            node_id
        } else {
            node_id
        };

        let now = Utc::now();
        let state = NodeState {
            node_id: final_id.clone(),
            region,
            public_ip,
            ports,
            capabilities,
            metrics: NodeMetrics::default(),
            status: NodeStatus::Registering,
            registered_at: now,
            last_heartbeat: now,
            config_version: 0,
        };

        self.nodes.insert(final_id.clone(), state);
        self.senders.insert(final_id.clone(), sender);

        info!("节点已注册: {}", final_id);

        final_id
    }

    /// 注销节点
    pub fn unregister(&self, node_id: &str) {
        self.nodes.remove(node_id);
        self.senders.remove(node_id);
        info!("节点已注销: {}", node_id);
    }

    /// 更新节点心跳
    pub fn update_heartbeat(&self, node_id: &str, metrics: NodeMetrics) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.metrics = metrics;
            node.last_heartbeat = Utc::now();
            node.status = NodeStatus::Active;
            tracing::trace!("节点 {} 心跳更新", node_id);
        }
    }

    /// 更新节点配置版本
    pub fn update_config_version(&self, node_id: &str, version: u64) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.config_version = version;
        }
    }

    /// 获取所有健康节点
    pub fn get_healthy_nodes(&self) -> Vec<NodeState> {
        let timeout = Duration::seconds(self.heartbeat_timeout_secs as i64);
        let now = Utc::now();

        self.nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Active)
            .filter(|n| now - n.last_heartbeat < timeout)
            .map(|n| n.clone())
            .collect()
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取健康节点数量
    pub fn healthy_node_count(&self) -> usize {
        self.get_healthy_nodes().len()
    }

    /// 向特定节点发送消息
    pub fn send_to_node(&self, node_id: &str, msg: &CoordinatorMessage) -> bool {
        if let Some(sender) = self.senders.get(node_id) {
            let json = msg.to_json();
            if sender.send(Message::Text(json.into())).is_ok() {
                return true;
            } else {
                warn!("发送消息到节点 {} 失败", node_id);
            }
        }
        false
    }

    /// 广播消息到所有节点
    pub fn broadcast(&self, msg: &CoordinatorMessage) {
        let json = msg.to_json();
        for sender in self.senders.iter() {
            let _ = sender.send(Message::Text(json.clone().into()));
        }
    }

    /// 广播配置更新
    pub fn broadcast_config(&self, config: TurnConfig) -> u64 {
        let version = self.config_version.fetch_add(1, Ordering::SeqCst) + 1;

        let msg = CoordinatorMessage::Config { version, config };

        self.broadcast(&msg);

        info!("配置已广播到 {} 个节点 (版本: {})", self.nodes.len(), version);

        version
    }

    /// 获取当前配置版本
    pub fn get_config_version(&self) -> u64 {
        self.config_version.load(Ordering::SeqCst)
    }

    /// 清理不健康的节点
    pub fn cleanup_unhealthy_nodes(&self) {
        let timeout = Duration::seconds(self.heartbeat_timeout_secs as i64);
        let now = Utc::now();

        let unhealthy: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| now - n.last_heartbeat >= timeout)
            .map(|n| n.node_id.clone())
            .collect();

        for node_id in unhealthy {
            warn!("节点 {} 心跳超时，标记为不健康", node_id);
            if let Some(mut node) = self.nodes.get_mut(&node_id) {
                node.status = NodeStatus::Unhealthy;
            }
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new(30)
    }
}

