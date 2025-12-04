//! WebSocket 连接处理
//!
//! 处理 WebSocket 连接的建立、消息收发、心跳、断开等

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::auth::models::AccessTokenClaims;
use crate::websocket::handlers::WsState;
use crate::websocket::models::{ClientMessage, ServerMessage, SourceType};

/// 心跳间隔（秒）
const HEARTBEAT_INTERVAL: u64 = 30;
/// 客户端超时（秒）
const CLIENT_TIMEOUT: u64 = 60;

/// 处理 WebSocket 连接
pub async fn handle_socket(socket: WebSocket, claims: AccessTokenClaims, state: WsState) {
    let user_id = claims.sub.clone();
    let device_id = claims.device_id.clone();

    info!(
        user_id = %user_id,
        device_id = %device_id,
        "WebSocket connection established"
    );

    // 创建消息通道
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 注册连接
    state
        .connection_manager
        .register(&user_id, &device_id, tx.clone());

    // 分离 WebSocket 读写
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 发送连接成功消息和未读摘要
    if let Ok(unread_summary) = state.unread_service.get_unread_summary(&user_id).await {
        let connected_msg = ServerMessage::Connected { unread_summary };
        let json = connected_msg.to_json();
        if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
            error!(user_id = %user_id, "Failed to send connected message: {}", e);
        }
    }

    // 克隆用于任务
    let user_id_clone = user_id.clone();
    let device_id_clone = device_id.clone();
    let state_clone = state.clone();

    // 发送任务：从通道接收消息并发送到 WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 心跳任务
    let tx_heartbeat = tx.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut heartbeat_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL));
        loop {
            heartbeat_interval.tick().await;
            let pong = ServerMessage::Pong {
                timestamp: Utc::now(),
            };
            if tx_heartbeat.send(Message::Text(pong.to_json().into())).is_err() {
                break;
            }
        }
    });

    // 接收任务：从 WebSocket 接收消息并处理
    let receive_task = tokio::spawn(async move {
        let mut last_activity = std::time::Instant::now();

        loop {
            // 带超时的接收
            match timeout(Duration::from_secs(CLIENT_TIMEOUT), ws_receiver.next()).await {
                Ok(Some(Ok(msg))) => {
                    last_activity = std::time::Instant::now();

                    match msg {
                        Message::Text(text) => {
                            handle_client_message(
                                &text,
                                &user_id_clone,
                                &device_id_clone,
                                &state_clone,
                            )
                            .await;
                        }
                        Message::Ping(data) => {
                            // 回复 Pong
                            let _ = tx.send(Message::Pong(data));
                        }
                        Message::Pong(_) => {
                            // 客户端响应心跳
                            debug!(user_id = %user_id_clone, "Received pong");
                        }
                        Message::Close(_) => {
                            info!(user_id = %user_id_clone, "Client initiated close");
                            break;
                        }
                        Message::Binary(_) => {
                            // 不处理二进制消息
                            warn!(user_id = %user_id_clone, "Received unexpected binary message");
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    error!(user_id = %user_id_clone, "WebSocket receive error: {}", e);
                    break;
                }
                Ok(None) => {
                    // 连接已关闭
                    info!(user_id = %user_id_clone, "WebSocket connection closed");
                    break;
                }
                Err(_) => {
                    // 超时，检查最后活动时间
                    if last_activity.elapsed() > Duration::from_secs(CLIENT_TIMEOUT) {
                        warn!(user_id = %user_id_clone, "Client timeout, closing connection");
                        break;
                    }
                }
            }
        }
    });

    // 等待任何一个任务完成
    tokio::select! {
        _ = send_task => {},
        _ = heartbeat_task => {},
        _ = receive_task => {},
    }

    // 注销连接
    state.connection_manager.unregister(&user_id, &device_id);

    info!(
        user_id = %user_id,
        device_id = %device_id,
        "WebSocket connection closed"
    );
}

/// 处理客户端消息
async fn handle_client_message(
    text: &str,
    user_id: &str,
    device_id: &str,
    state: &WsState,
) {
    let message = match ClientMessage::from_json(text) {
        Ok(msg) => msg,
        Err(e) => {
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to parse client message"
            );
            // 发送错误响应
            let error_msg = ServerMessage::Error {
                code: "invalid_message".to_string(),
                message: "Failed to parse message".to_string(),
            };
            state.connection_manager.send_to_device(user_id, device_id, &error_msg);
            return;
        }
    };

    match message {
        ClientMessage::Ping => {
            // 客户端主动 ping，回复 pong
            let pong = ServerMessage::Pong {
                timestamp: Utc::now(),
            };
            state.connection_manager.send_to_device(user_id, device_id, &pong);
        }

        ClientMessage::MarkRead { target_type, target_id } => {
            handle_mark_read(user_id, device_id, target_type, &target_id, state).await;
        }

        ClientMessage::SubscribePresence { user_ids: _ } => {
            // TODO: 实现在线状态订阅（预留功能）
            debug!(user_id = %user_id, "Presence subscription not implemented yet");
        }
    }
}

/// 处理标记已读
async fn handle_mark_read(
    user_id: &str,
    device_id: &str,
    target_type: SourceType,
    target_id: &str,
    state: &WsState,
) {
    let result = match target_type {
        SourceType::Friend => {
            // 标记好友消息已读
            state.unread_service.mark_friend_read(user_id, target_id).await
        }
        SourceType::Group => {
            // 解析群 ID
            let group_id = match Uuid::parse_str(target_id) {
                Ok(id) => id,
                Err(_) => {
                    let error_msg = ServerMessage::Error {
                        code: "invalid_group_id".to_string(),
                        message: "Invalid group ID format".to_string(),
                    };
                    state.connection_manager.send_to_device(user_id, device_id, &error_msg);
                    return;
                }
            };
            state.unread_service.mark_group_read(user_id, &group_id).await
        }
    };

    if let Err(e) = result {
        error!(
            user_id = %user_id,
            target_type = %target_type,
            target_id = %target_id,
            error = %e,
            "Failed to mark read"
        );
        let error_msg = ServerMessage::Error {
            code: "mark_read_failed".to_string(),
            message: "Failed to mark messages as read".to_string(),
        };
        state.connection_manager.send_to_device(user_id, device_id, &error_msg);
        return;
    }

    // 同步到用户的其他设备
    state.connection_manager.send_to_other_devices(
        user_id,
        device_id,
        &ServerMessage::ReadSync {
            source_type: target_type,
            source_id: target_id.to_string(),
            reader_id: user_id.to_string(),
            read_at: Utc::now(),
        },
    );

    // 如果是好友消息，通知对方已读（仅当功能开启时）
    if target_type == SourceType::Friend {
        if let Err(e) = state
            .notification_service
            .notify_read_sync(target_type, target_id, user_id, target_id)
            .await
        {
            debug!(
                user_id = %user_id,
                friend_id = %target_id,
                error = %e,
                "Failed to send read sync notification"
            );
        }
    }

    debug!(
        user_id = %user_id,
        target_type = %target_type,
        target_id = %target_id,
        "Messages marked as read"
    );
}

