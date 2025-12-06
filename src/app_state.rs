//! 应用全局状态管理
//!
//! 统一管理所有服务实例，避免在 main.rs 中创建大量分散的 State 对象

use crate::auth::{
    handlers::{DeviceState, LoginState, LogoutState, RefreshTokenState, RegisterState},
    middleware::AuthState,
    services::{BlacklistService, DeviceService, TokenService},
    utils::KeyManager,
};
use crate::config::turn_config;
use crate::friends::handlers::FriendsState;
use crate::friends_messages::handlers::MessagesState;
use crate::groups::handlers::GroupsState;
use crate::group_messages::handlers::GroupMessagesState;
use crate::profile::handlers::routes::ProfileAppState;
use crate::storage::S3Client;
use crate::turn::{
    handlers::TurnState,
    services::{CredentialService, LoadBalancer, NodeRegistry, SecretManager},
};
use crate::webrtc_room::{RoomManager, RoomTokenService, WebRTCState};
use crate::websocket::{
    handlers::WsState,
    services::{ConnectionManager, NotificationService, UnreadService},
};
use sqlx::PgPool;
use std::sync::Arc;

/// 应用全局状态
///
/// 集中管理所有服务实例，方便在各模块间共享
#[derive(Clone)]
pub struct AppState {
    /// 数据库连接池
    pub db: PgPool,

    /// Token 服务（JWT 签发与验证）
    pub token_service: Arc<TokenService>,

    /// 黑名单服务（Token 撤销管理）
    pub blacklist_service: Arc<BlacklistService>,

    /// 设备服务（多设备管理）
    pub device_service: Arc<DeviceService>,

    /// S3/MinIO 客户端
    pub s3_client: Arc<S3Client>,

    /// API 基础 URL
    pub api_base_url: String,

    /// WebSocket 连接管理器
    pub connection_manager: Arc<ConnectionManager>,

    /// WebSocket 通知服务
    pub notification_service: NotificationService,

    /// TURN 节点注册中心
    pub turn_node_registry: Arc<NodeRegistry>,

    /// TURN 密钥管理器
    pub turn_secret_manager: Arc<SecretManager>,

    /// TURN 凭证服务
    pub turn_credential_service: Arc<CredentialService>,

    /// WebRTC 房间管理器
    pub room_manager: Arc<RoomManager>,

    /// WebRTC 房间 Token 服务
    pub room_token_service: Arc<RoomTokenService>,
}

impl AppState {
    /// 创建应用状态
    ///
    /// # Arguments
    /// * `db` - 数据库连接池
    /// * `key_manager` - RSA 密钥管理器
    /// * `s3_client` - S3/MinIO 客户端
    /// * `api_base_url` - API 基础 URL
    pub fn new(
        db: PgPool,
        key_manager: KeyManager,
        s3_client: Arc<S3Client>,
        api_base_url: String,
    ) -> Self {
        let token_service = Arc::new(TokenService::new(key_manager, db.clone()));
        let blacklist_service = Arc::new(BlacklistService::new(db.clone()));
        let device_service = Arc::new(DeviceService::new(db.clone()));

        // WebSocket 相关服务
        let connection_manager = Arc::new(ConnectionManager::new());
        let notification_service =
            NotificationService::new(db.clone(), connection_manager.clone());

        // TURN 相关服务
        let turn_cfg = turn_config();
        let turn_node_registry = Arc::new(NodeRegistry::new(turn_cfg.heartbeat_timeout_secs));
        let turn_secret_manager = Arc::new(SecretManager::new(turn_cfg.secret_rotation_hours));
        let turn_credential_service = Arc::new(CredentialService::new(
            turn_secret_manager.clone(),
            turn_cfg.credential_ttl_secs,
            turn_cfg.realm.clone(),
        ));

        // 如果启用 TURN，启动密钥轮换任务
        if turn_cfg.enabled {
            turn_secret_manager
                .clone()
                .start_rotation_task(turn_node_registry.clone());
        }

        // WebRTC 房间相关服务
        let room_manager = Arc::new(RoomManager::new());
        // 房间 Token 使用 TURN 的 agent_auth_token 作为密钥，有效期 10 分钟
        let room_token_service = Arc::new(RoomTokenService::new(
            turn_cfg.agent_auth_token.clone(),
            600,
        ));

        Self {
            db,
            token_service,
            blacklist_service,
            device_service,
            s3_client,
            api_base_url,
            connection_manager,
            notification_service,
            turn_node_registry,
            turn_secret_manager,
            turn_credential_service,
            room_manager,
            room_token_service,
        }
    }

    // ========================================
    // 便捷方法：生成各模块所需的 State
    // ========================================

    /// 获取注册处理器状态
    pub fn register_state(&self) -> RegisterState {
        RegisterState {
            db: self.db.clone(),
            token_service: self.token_service.clone(),
        }
    }

    /// 获取登录处理器状态
    pub fn login_state(&self) -> LoginState {
        LoginState {
            db: self.db.clone(),
            token_service: self.token_service.clone(),
        }
    }

    /// 获取刷新 Token 处理器状态
    pub fn refresh_state(&self) -> RefreshTokenState {
        RefreshTokenState {
            token_service: self.token_service.clone(),
        }
    }

    /// 获取登出处理器状态
    pub fn logout_state(&self) -> LogoutState {
        LogoutState {
            token_service: self.token_service.clone(),
            blacklist_service: self.blacklist_service.clone(),
        }
    }

    /// 获取设备管理处理器状态
    pub fn device_state(&self) -> DeviceState {
        DeviceState {
            device_service: self.device_service.clone(),
            blacklist_service: self.blacklist_service.clone(),
        }
    }

    /// 获取认证中间件状态
    pub fn auth_state(&self) -> AuthState {
        AuthState {
            token_service: self.token_service.clone(),
            blacklist_service: self.blacklist_service.clone(),
            db: self.db.clone(),
        }
    }

    /// 获取好友模块状态
    pub fn friends_state(&self) -> FriendsState {
        FriendsState::new(self.db.clone())
    }


    /// 获取消息模块状态
    pub fn messages_state(&self) -> MessagesState {
        MessagesState::with_notification(self.db.clone(), self.notification_service.clone())
    }

    /// 获取个人资料模块状态
    pub fn profile_state(&self) -> ProfileAppState {
        ProfileAppState {
            profile_service: crate::profile::services::ProfileService::new(self.db.clone()),
            s3_client: self.s3_client.clone(),
            blacklist_service: self.blacklist_service.clone(),
        }
    }

    /// 获取群聊模块状态
    pub fn groups_state(&self) -> GroupsState {
        GroupsState::new(self.db.clone(), self.s3_client.clone())
    }

    /// 获取群消息模块状态
    pub fn group_messages_state(&self) -> GroupMessagesState {
        GroupMessagesState::with_notification(self.db.clone(), self.notification_service.clone())
    }

    /// 获取 WebSocket 模块状态
    pub fn ws_state(&self) -> WsState {
        WsState::new(
            self.connection_manager.clone(),
            self.notification_service.clone(),
            UnreadService::new(self.db.clone()),
            self.token_service.clone(),
        )
    }

    /// 获取通知服务（供其他模块使用）
    pub fn notification_service(&self) -> &NotificationService {
        &self.notification_service
    }

    /// 获取连接管理器（供其他模块使用）
    pub fn connection_manager(&self) -> &Arc<ConnectionManager> {
        &self.connection_manager
    }

    /// 获取 TURN 模块状态
    pub fn turn_state(&self) -> TurnState {
        let turn_cfg = turn_config();
        TurnState::new(
            self.turn_node_registry.clone(),
            self.turn_secret_manager.clone(),
            self.turn_credential_service.clone(),
            turn_cfg.agent_auth_token.clone(),
            turn_cfg.enabled,
        )
    }

    /// 获取 WebRTC 房间模块状态
    pub fn webrtc_state(&self) -> WebRTCState {
        let turn_cfg = turn_config();
        let load_balancer = Arc::new(LoadBalancer::new(self.turn_node_registry.clone()));

        WebRTCState::new(
            self.room_manager.clone(),
            self.room_token_service.clone(),
            load_balancer,
            self.turn_credential_service.clone(),
            turn_cfg.enabled,
        )
    }
}

