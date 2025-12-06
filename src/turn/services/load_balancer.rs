//! 负载均衡服务
//!
//! 智能选择最优 TURN 节点

use std::sync::Arc;

use crate::turn::models::{NodeState, NodeStatus, SelectedNode};

use super::NodeRegistry;

/// 负载均衡器
pub struct LoadBalancer {
    /// 节点注册中心
    registry: Arc<NodeRegistry>,
}

impl LoadBalancer {
    /// 创建负载均衡器
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// 为客户端选择最优节点
    ///
    /// # Arguments
    /// * `client_region` - 客户端区域（可选）
    /// * `count` - 返回节点数量
    pub fn select_nodes(
        &self,
        client_region: Option<&str>,
        count: usize,
    ) -> Vec<SelectedNode> {
        let healthy_nodes = self.registry.get_healthy_nodes();

        if healthy_nodes.is_empty() {
            return vec![];
        }

        // 计算每个节点的分数
        let mut scored: Vec<_> = healthy_nodes
            .into_iter()
            .filter(|n| n.status == NodeStatus::Active)
            .map(|node| {
                let score = self.calculate_score(&node, client_region);
                (node, score)
            })
            .collect();

        // 按分数降序排序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 返回前 N 个节点
        scored
            .into_iter()
            .take(count)
            .map(|(node, score)| SelectedNode {
                node_id: node.node_id.clone(),
                region: node.region.clone(),
                urls: node.build_turn_urls(),
                score,
            })
            .collect()
    }

    /// 计算节点评分
    fn calculate_score(&self, node: &NodeState, client_region: Option<&str>) -> f64 {
        let mut score = 100.0;

        // 1. 区域匹配加分 (+30 完全匹配, +15 相近)
        if let Some(region) = client_region {
            if node.region == region {
                score += 30.0;
            } else if self.is_nearby_region(&node.region, region) {
                score += 15.0;
            }
        }

        // 2. CPU 负载扣分 (-0.5 per %)
        score -= node.metrics.cpu_percent as f64 * 0.5;

        // 3. 内存负载扣分 (-0.3 per %)
        score -= node.metrics.memory_percent as f64 * 0.3;

        // 4. 活跃会话数扣分 (-0.1 per session, max -20)
        score -= (node.metrics.active_sessions as f64 * 0.1).min(20.0);

        // 5. 带宽使用率扣分
        if node.capabilities.max_bandwidth_mbps > 0 {
            let bandwidth_percent = (node.metrics.bandwidth_out_mbps
                / node.capabilities.max_bandwidth_mbps as f64)
                * 100.0;
            score -= bandwidth_percent * 0.3;
        }

        score.max(0.0)
    }

    /// 判断两个区域是否相近
    fn is_nearby_region(&self, region1: &str, region2: &str) -> bool {
        // 区域相近规则
        let nearby_groups = [
            vec!["cn-north", "cn-east", "cn-south"],
            vec!["us-west", "us-east"],
            vec!["eu-west", "eu-central"],
            vec!["ap-southeast", "ap-northeast"],
        ];

        for group in &nearby_groups {
            if group.contains(&region1) && group.contains(&region2) {
                return true;
            }
        }

        false
    }
}

