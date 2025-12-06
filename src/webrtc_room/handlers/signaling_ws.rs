//! 信令 WebSocket 处理

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::webrtc_room::models::{ClientSignaling, Participant, ServerSignaling};
use crate::webrtc_room::services::TokenError;

use super::WebRTCState;

/// WebSocket 连接参数
#[derive(Debug, Deserialize)]
pub struct WsParams {
    /// 房间 Token
    pub token: String,
}

/// 信令 WebSocket 处理入口
///
/// GET /ws/webrtc/rooms/{room_id}?token=xxx
pub async fn signaling_ws_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    Query(params): Query<WsParams>,
    State(state): State<WebRTCState>,
) -> Response {
    // 验证 Token
    let claims = match state.token_service.verify_token(&params.token) {
        Ok(claims) => claims,
        Err(e) => {
            let msg = match e {
                TokenError::InvalidFormat | TokenError::InvalidSignature => "Token 无效",
                TokenError::Expired => "Token 已过期",
            };
            return Response::builder()
                .status(401)
                .body(msg.into())
                .unwrap();
        }
    };

    // 验证房间 ID 匹配
    if claims.room_id != room_id {
        return Response::builder()
            .status(403)
            .body("Token 与房间不匹配".into())
            .unwrap();
    }

    // 验证房间是否存在
    if state.room_manager.get_room(&room_id).is_none() {
        return Response::builder()
            .status(404)
            .body("房间不存在".into())
            .unwrap();
    }

    ws.on_upgrade(move |socket| handle_signaling(socket, state, claims.participant_id, claims.room_id, claims.display_name, claims.is_creator, claims.user_id))
}

/// 处理信令 WebSocket 连接
async fn handle_signaling(
    socket: WebSocket,
    state: WebRTCState,
    participant_id: String,
    room_id: String,
    display_name: String,
    is_creator: bool,
    user_id: Option<String>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息通道
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 创建参与者信息
    let participant = Participant {
        participant_id: participant_id.clone(),
        display_name: display_name.clone(),
        user_id,
        is_creator,
        joined_at: Utc::now(),
    };

    // 添加到房间
    let existing_participants = match state
        .room_manager
        .add_participant(&room_id, participant, tx.clone())
    {
        Some(participants) => participants,
        None => {
            error!(room_id = %room_id, "房间不存在，无法加入");
            let _ = ws_sender.close().await;
            return;
        }
    };

    info!(
        room_id = %room_id,
        participant_id = %participant_id,
        display_name = %display_name,
        "参与者已连接信令"
    );

    // 发送加入成功消息
    let joined_msg = ServerSignaling::Joined {
        participant_id: participant_id.clone(),
        participants: existing_participants,
    };
    let _ = tx.send(Message::Text(joined_msg.to_json().into()));

    // 发送任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 接收并处理消息
    let room_id_clone = room_id.clone();
    let participant_id_clone = participant_id.clone();
    let state_clone = state.clone();

    let receive_task = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    handle_client_message(
                        &text,
                        &room_id_clone,
                        &participant_id_clone,
                        &state_clone,
                    )
                    .await;
                }
                Ok(Message::Ping(_data)) => {
                    let _ = state_clone
                        .room_manager
                        .send_to_participant(&room_id_clone, &participant_id_clone, &ServerSignaling::Error {
                            code: "pong".to_string(),
                            message: "".to_string(),
                        });
                }
                Ok(Message::Close(_)) => {
                    info!(
                        room_id = %room_id_clone,
                        participant_id = %participant_id_clone,
                        "客户端主动关闭连接"
                    );
                    break;
                }
                Err(e) => {
                    error!(
                        room_id = %room_id_clone,
                        participant_id = %participant_id_clone,
                        error = %e,
                        "WebSocket 错误"
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    // 等待任何一个任务完成
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
    }

    // 清理：从房间移除参与者
    state.room_manager.remove_participant(&room_id, &participant_id);

    info!(
        room_id = %room_id,
        participant_id = %participant_id,
        "参与者已断开连接"
    );
}

/// 处理客户端消息
async fn handle_client_message(
    text: &str,
    room_id: &str,
    from_id: &str,
    state: &WebRTCState,
) {
    let message = match ClientSignaling::from_json(text) {
        Ok(msg) => msg,
        Err(e) => {
            warn!(
                room_id = %room_id,
                from = %from_id,
                error = %e,
                "解析信令消息失败"
            );
            let error_msg = ServerSignaling::Error {
                code: "invalid_message".to_string(),
                message: "消息格式无效".to_string(),
            };
            state.room_manager.send_to_participant(room_id, from_id, &error_msg);
            return;
        }
    };

    match message {
        ClientSignaling::Offer { to, sdp } => {
            let forward = ServerSignaling::Offer {
                from: from_id.to_string(),
                sdp,
            };
            state.room_manager.forward_signaling(room_id, from_id, &to, forward);
        }

        ClientSignaling::Answer { to, sdp } => {
            let forward = ServerSignaling::Answer {
                from: from_id.to_string(),
                sdp,
            };
            state.room_manager.forward_signaling(room_id, from_id, &to, forward);
        }

        ClientSignaling::Candidate { to, candidate } => {
            let forward = ServerSignaling::Candidate {
                from: from_id.to_string(),
                candidate,
            };
            state.room_manager.forward_signaling(room_id, from_id, &to, forward);
        }

        ClientSignaling::Leave => {
            debug!(
                room_id = %room_id,
                participant_id = %from_id,
                "参与者发送离开消息"
            );
            // 实际的清理在连接断开时处理
        }
    }
}

