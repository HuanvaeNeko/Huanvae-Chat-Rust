//! 主服务器通信模块
//!
//! 通过 WebSocket 与主服务器保持连接

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
};
use url::Url;

use crate::protocol::{AgentMessage, CoordinatorMessage};

/// WebSocket 发送端类型
type WsSender = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

/// WebSocket 接收端类型
type WsReceiver = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
>;

/// 主服务器客户端
#[derive(Clone)]
pub struct CoordinatorClient {
    sender: Arc<Mutex<WsSender>>,
    receiver: Arc<Mutex<WsReceiver>>,
}

/// 连接错误
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("URL 解析失败: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("WebSocket 连接失败: {0}")]
    WebSocket(#[from] WsError),

    #[error("发送消息失败: {0}")]
    Send(String),

    #[error("接收消息失败: {0}")]
    Receive(String),

    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
}

impl CoordinatorClient {
    /// 连接到主服务器
    pub async fn connect(url: &str, token: &str) -> Result<Self, ConnectError> {
        // 构建带 token 的 URL
        let mut parsed_url = Url::parse(url)?;
        parsed_url.query_pairs_mut().append_pair("token", token);

        tracing::debug!("连接到: {}", parsed_url.as_str());

        // 建立 WebSocket 连接
        let (ws_stream, response) = connect_async(parsed_url.as_str()).await?;

        tracing::debug!("WebSocket 连接建立, 响应状态: {}", response.status());

        // 分离读写
        let (sender, receiver) = ws_stream.split();

        Ok(Self {
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
        })
    }

    /// 发送消息到服务器
    pub async fn send(&self, msg: &AgentMessage) -> Result<(), ConnectError> {
        let json = serde_json::to_string(msg)?;
        tracing::trace!("发送消息: {}", json);

        let mut sender = self.sender.lock().await;
        sender
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| ConnectError::Send(e.to_string()))?;

        Ok(())
    }

    /// 接收服务器消息
    pub async fn recv(&self) -> Result<Option<CoordinatorMessage>, ConnectError> {
        let mut receiver = self.receiver.lock().await;

        while let Some(msg_result) = receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    tracing::trace!("收到消息: {}", text);
                    let parsed = CoordinatorMessage::from_json(&text)?;
                    return Ok(Some(parsed));
                }
                Ok(Message::Ping(data)) => {
                    // 自动响应 Pong
                    drop(receiver);
                    let mut sender = self.sender.lock().await;
                    let _ = sender.send(Message::Pong(data)).await;
                    receiver = self.receiver.lock().await;
                }
                Ok(Message::Pong(_)) => {
                    // 忽略 Pong
                }
                Ok(Message::Close(frame)) => {
                    tracing::info!("服务器关闭连接: {:?}", frame);
                    return Ok(None);
                }
                Ok(Message::Binary(_)) => {
                    tracing::warn!("收到未预期的二进制消息");
                }
                Ok(Message::Frame(_)) => {
                    // 忽略原始帧
                }
                Err(e) => {
                    return Err(ConnectError::Receive(e.to_string()));
                }
            }
        }

        Ok(None)
    }
}

