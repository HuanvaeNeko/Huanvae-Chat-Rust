//! 应用全局状态管理
//!
//! 统一管理所有服务实例，避免在 main.rs 中创建大量分散的 State 对象

use crate::auth::{
    handlers::{DeviceState, LoginState, LogoutState, RefreshTokenState, RegisterState},
    middleware::AuthState,
    services::{BlacklistService, DeviceService, TokenService},
    utils::KeyManager,
};
use crate::friends::services::FriendsState;
use crate::friends_messages::handlers::MessagesState;
use crate::profile::handlers::routes::ProfileAppState;
use crate::storage::S3Client;
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

        Self {
            db,
            token_service,
            blacklist_service,
            device_service,
            s3_client,
            api_base_url,
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
        MessagesState::new(self.db.clone())
    }

    /// 获取个人资料模块状态
    pub fn profile_state(&self) -> ProfileAppState {
        ProfileAppState {
            profile_service: crate::profile::services::ProfileService::new(self.db.clone()),
            s3_client: self.s3_client.clone(),
            blacklist_service: self.blacklist_service.clone(),
        }
    }
}

