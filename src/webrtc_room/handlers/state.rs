//! WebRTC 房间模块状态

use std::sync::Arc;

use crate::turn::services::{CredentialService, LoadBalancer};
use crate::webrtc_room::services::{RoomManager, RoomService, RoomTokenService};

/// WebRTC 房间模块状态
#[derive(Clone)]
pub struct WebRTCState {
    /// 房间服务
    pub room_service: Arc<RoomService>,
    /// 房间管理器
    pub room_manager: Arc<RoomManager>,
    /// Token 服务
    pub token_service: Arc<RoomTokenService>,
    /// 负载均衡器
    pub load_balancer: Arc<LoadBalancer>,
    /// 凭证服务
    pub credential_service: Arc<CredentialService>,
    /// TURN 是否启用
    pub turn_enabled: bool,
}

impl WebRTCState {
    /// 创建 WebRTC 状态
    pub fn new(
        room_manager: Arc<RoomManager>,
        token_service: Arc<RoomTokenService>,
        load_balancer: Arc<LoadBalancer>,
        credential_service: Arc<CredentialService>,
        turn_enabled: bool,
    ) -> Self {
        let room_service = Arc::new(RoomService::new(room_manager.clone()));

        Self {
            room_service,
            room_manager,
            token_service,
            load_balancer,
            credential_service,
            turn_enabled,
        }
    }
}

