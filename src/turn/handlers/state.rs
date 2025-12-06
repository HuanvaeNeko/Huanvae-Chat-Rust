//! TURN 模块状态

use std::sync::Arc;

use crate::turn::services::{CredentialService, LoadBalancer, NodeRegistry, SecretManager};

/// TURN 模块状态
#[derive(Clone)]
pub struct TurnState {
    /// 节点注册中心
    pub node_registry: Arc<NodeRegistry>,
    /// 密钥管理器
    pub secret_manager: Arc<SecretManager>,
    /// 负载均衡器
    pub load_balancer: Arc<LoadBalancer>,
    /// 凭证服务
    pub credential_service: Arc<CredentialService>,
    /// Agent 认证令牌
    pub agent_auth_token: String,
    /// 是否启用 TURN 功能
    pub enabled: bool,
}

impl TurnState {
    /// 创建 TURN 状态
    pub fn new(
        node_registry: Arc<NodeRegistry>,
        secret_manager: Arc<SecretManager>,
        credential_service: Arc<CredentialService>,
        agent_auth_token: String,
        enabled: bool,
    ) -> Self {
        let load_balancer = Arc::new(LoadBalancer::new(node_registry.clone()));

        Self {
            node_registry,
            secret_manager,
            load_balancer,
            credential_service,
            agent_auth_token,
            enabled,
        }
    }
}

