//! 加入房间 API

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::common::{ApiResponse, AppError};
use crate::webrtc_room::models::{IceServerConfig, JoinRoomRequest, JoinRoomResponse};
use crate::webrtc_room::services::RoomError;

use super::WebRTCState;

/// 加入房间
///
/// POST /api/webrtc/rooms/{room_id}/join
/// 无需登录，仅需房间密码
pub async fn join_room(
    State(state): State<WebRTCState>,
    Path(room_id): Path<String>,
    Json(request): Json<JoinRoomRequest>,
) -> Result<Json<ApiResponse<JoinRoomResponse>>, AppError> {
    // 验证房间和密码
    let room_state = state
        .room_service
        .verify_password(&room_id, &request.password)
        .map_err(|e| match e {
            RoomError::NotFound => AppError::NotFound("房间不存在".to_string()),
            RoomError::InvalidPassword => AppError::Unauthorized,
            RoomError::Expired => AppError::BadRequest("房间已过期".to_string()),
            RoomError::RoomFull => AppError::BadRequest("房间已满".to_string()),
            RoomError::NotCreator => AppError::Forbidden,
        })?;

    // 生成参与者 ID
    let participant_id = format!("p_{}", &Uuid::new_v4().to_string()[..8]);

    // 生成 WebSocket Token
    let claims = state.token_service.create_claims(
        participant_id.clone(),
        room_id.clone(),
        request.display_name.clone(),
        false, // 不是创建者
        None,  // 访客没有 user_id
    );
    let ws_token = state.token_service.generate_token(&claims);
    let token_expires_at = state.token_service.get_expires_at();

    // 获取 ICE 服务器配置
    let ice_servers = if state.turn_enabled {
        // 选择最优 TURN 节点
        let selected_nodes = state.load_balancer.select_nodes(None, 3);

        if selected_nodes.is_empty() {
            // 没有 TURN 节点，只返回 STUN
            vec![IceServerConfig {
                urls: vec![
                    "stun:stun.l.google.com:19302".to_string(),
                    "stun:stun1.l.google.com:19302".to_string(),
                ],
                username: None,
                credential: None,
            }]
        } else {
            // 生成 TURN 凭证
            let guest_id = format!("guest_{}", participant_id);
            let ice_response = state
                .credential_service
                .generate_ice_servers(&guest_id, selected_nodes)
                .await;

            ice_response
                .ice_servers
                .into_iter()
                .map(|s| IceServerConfig {
                    urls: s.urls,
                    username: s.username,
                    credential: s.credential,
                })
                .collect()
        }
    } else {
        // TURN 未启用，只返回公共 STUN
        vec![IceServerConfig {
            urls: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            username: None,
            credential: None,
        }]
    };

    tracing::info!(
        room_id = %room_id,
        participant_id = %participant_id,
        display_name = %request.display_name,
        "参与者准备加入房间"
    );

    Ok(Json(ApiResponse::success(JoinRoomResponse {
        participant_id,
        ws_token,
        room_name: room_state.room.name,
        ice_servers,
        token_expires_at,
    })))
}

