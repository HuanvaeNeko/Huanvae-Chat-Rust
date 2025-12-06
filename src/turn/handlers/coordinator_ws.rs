//! Agent WebSocket 连接处理

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::turn::models::protocol::{AgentMessage, CoordinatorMessage, TurnConfig};

use super::TurnState;

/// WebSocket 连接参数
#[derive(Debug, Deserialize)]
pub struct WsParams {
    /// Agent 认证令牌
    pub token: String,
}

/// Agent WebSocket 连接处理
///
/// GET /internal/turn-coordinator?token=xxx
pub async fn coordinator_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<TurnState>,
    Query(params): Query<WsParams>,
) -> Response {
    // 验证 token
    if params.token != state.agent_auth_token {
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }

    ws.on_upgrade(move |socket| handle_agent_connection(socket, state))
}

/// 处理 Agent 连接
async fn handle_agent_connection(socket: WebSocket, state: TurnState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息通道
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 发送任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut node_id: Option<String> = None;

    // 接收并处理消息
    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                match AgentMessage::from_json(&text) {
                    Ok(agent_msg) => {
                        match agent_msg {
                            AgentMessage::Register {
                                node_id: req_node_id,
                                region,
                                public_ip,
                                ports,
                                capabilities,
                            } => {
                                // 注册节点
                                let registered_id = state.node_registry.register(
                                    req_node_id.clone(),
                                    region,
                                    public_ip,
                                    ports,
                                    capabilities,
                                    tx.clone(),
                                );

                                node_id = Some(registered_id.clone());

                                // 发送注册确认
                                let response = CoordinatorMessage::Registered {
                                    node_id: registered_id.clone(),
                                    assigned_id: None,
                                };
                                let _ = tx.send(Message::Text(response.to_json().into()));

                                // 下发初始配置
                                let secret = state.secret_manager.get_current_secret().await;
                                let config = TurnConfig {
                                    realm: state.credential_service.get_realm().to_string(),
                                    auth_secret: secret,
                                    total_quota: 0,
                                    user_quota: 0,
                                    max_bps: 0,
                                };

                                let config_msg = CoordinatorMessage::Config {
                                    version: state.node_registry.get_config_version(),
                                    config,
                                };
                                let _ = tx.send(Message::Text(config_msg.to_json().into()));

                                info!("节点 {} 已连接并配置", registered_id);
                            }

                            AgentMessage::Heartbeat { metrics } => {
                                if let Some(ref id) = node_id {
                                    state.node_registry.update_heartbeat(id, metrics);
                                }
                            }

                            AgentMessage::ConfigApplied {
                                config_version,
                                success,
                                error,
                            } => {
                                if let Some(ref id) = node_id {
                                    if success {
                                        state
                                            .node_registry
                                            .update_config_version(id, config_version);
                                        info!("节点 {} 配置应用成功 (v{})", id, config_version);
                                    } else {
                                        warn!(
                                            "节点 {} 配置应用失败: {:?}",
                                            id,
                                            error.unwrap_or_default()
                                        );
                                    }
                                }
                            }

                            AgentMessage::RequestConfig => {
                                // 重新下发配置
                                let secret = state.secret_manager.get_current_secret().await;
                                let config = TurnConfig {
                                    realm: state.credential_service.get_realm().to_string(),
                                    auth_secret: secret,
                                    total_quota: 0,
                                    user_quota: 0,
                                    max_bps: 0,
                                };

                                let config_msg = CoordinatorMessage::Config {
                                    version: state.node_registry.get_config_version(),
                                    config,
                                };
                                let _ = tx.send(Message::Text(config_msg.to_json().into()));
                            }
                        }
                    }
                    Err(e) => {
                        warn!("解析 Agent 消息失败: {}", e);
                        let error_msg = CoordinatorMessage::Error {
                            code: "invalid_message".to_string(),
                            message: e.to_string(),
                        };
                        let _ = tx.send(Message::Text(error_msg.to_json().into()));
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = tx.send(Message::Pong(data));
            }
            Ok(Message::Close(_)) => {
                info!("Agent 连接关闭");
                break;
            }
            Err(e) => {
                error!("WebSocket 错误: {}", e);
                break;
            }
            _ => {}
        }
    }

    // 清理
    if let Some(id) = node_id {
        state.node_registry.unregister(&id);
    }

    send_task.abort();
}

