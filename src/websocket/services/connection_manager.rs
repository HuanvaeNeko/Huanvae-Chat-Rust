//! WebSocket 连接管理器
//!
//! 管理所有 WebSocket 连接，支持多设备同时在线

use axum::extract::ws::Message;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::websocket::models::ServerMessage;

/// 连接信息
#[derive(Debug)]
pub struct ConnectionInfo {
    /// 设备 ID
    pub device_id: String,
    /// 消息发送通道
    pub sender: mpsc::UnboundedSender<Message>,
}

/// WebSocket 连接管理器
///
/// 使用 DashMap 实现高性能并发访问
#[derive(Debug)]
pub struct ConnectionManager {
    /// 用户连接映射：user_id -> Vec<ConnectionInfo>
    /// 支持同一用户多设备同时在线
    connections: DashMap<String, Vec<ConnectionInfo>>,
    /// 在线用户数（去重）
    online_users: AtomicUsize,
    /// 总连接数（包含多设备）
    total_connections: AtomicUsize,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            online_users: AtomicUsize::new(0),
            total_connections: AtomicUsize::new(0),
        }
    }

    /// 注册新连接
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `device_id` - 设备 ID
    /// * `sender` - 消息发送通道
    pub fn register(
        &self,
        user_id: &str,
        device_id: &str,
        sender: mpsc::UnboundedSender<Message>,
    ) {
        let conn_info = ConnectionInfo {
            device_id: device_id.to_string(),
            sender,
        };

        let mut entry = self.connections.entry(user_id.to_string()).or_default();
        let is_first_device = entry.is_empty();

        // 移除同设备的旧连接（如果存在）
        entry.retain(|c| c.device_id != device_id);
        entry.push(conn_info);

        if is_first_device {
            self.online_users.fetch_add(1, Ordering::SeqCst);
        }
        self.total_connections.fetch_add(1, Ordering::SeqCst);

        info!(
            user_id = %user_id,
            device_id = %device_id,
            "WebSocket connection registered"
        );
    }

    /// 移除连接
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `device_id` - 设备 ID
    pub fn unregister(&self, user_id: &str, device_id: &str) {
        let mut should_remove_user = false;

        if let Some(mut entry) = self.connections.get_mut(user_id) {
            let before_len = entry.len();
            entry.retain(|c| c.device_id != device_id);
            let after_len = entry.len();

            if before_len > after_len {
                self.total_connections.fetch_sub(1, Ordering::SeqCst);
            }

            if entry.is_empty() {
                should_remove_user = true;
            }
        }

        if should_remove_user {
            self.connections.remove(user_id);
            self.online_users.fetch_sub(1, Ordering::SeqCst);
        }

        info!(
            user_id = %user_id,
            device_id = %device_id,
            "WebSocket connection unregistered"
        );
    }

    /// 检查用户是否在线
    pub fn is_online(&self, user_id: &str) -> bool {
        self.connections
            .get(user_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// 向指定用户发送消息（所有设备）
    ///
    /// # Arguments
    /// * `user_id` - 目标用户 ID
    /// * `message` - 要发送的消息
    ///
    /// # Returns
    /// 成功发送的设备数量
    pub fn send_to_user(&self, user_id: &str, message: &ServerMessage) -> usize {
        let json = message.to_json();
        let mut sent_count = 0;

        if let Some(entry) = self.connections.get(user_id) {
            for conn in entry.iter() {
                if conn.sender.send(Message::Text(json.clone().into())).is_ok() {
                    sent_count += 1;
                } else {
                    warn!(
                        user_id = %user_id,
                        device_id = %conn.device_id,
                        "Failed to send message to device"
                    );
                }
            }
        }

        debug!(
            user_id = %user_id,
            sent_count = sent_count,
            "Message sent to user"
        );

        sent_count
    }

    /// 向指定用户的指定设备发送消息
    ///
    /// # Arguments
    /// * `user_id` - 目标用户 ID
    /// * `device_id` - 目标设备 ID
    /// * `message` - 要发送的消息
    ///
    /// # Returns
    /// 是否发送成功
    pub fn send_to_device(
        &self,
        user_id: &str,
        device_id: &str,
        message: &ServerMessage,
    ) -> bool {
        let json = message.to_json();

        if let Some(entry) = self.connections.get(user_id) {
            for conn in entry.iter() {
                if conn.device_id == device_id {
                    return conn.sender.send(Message::Text(json.into())).is_ok();
                }
            }
        }

        false
    }

    /// 向多个用户发送消息
    ///
    /// # Arguments
    /// * `user_ids` - 目标用户 ID 列表
    /// * `message` - 要发送的消息
    ///
    /// # Returns
    /// 总共成功发送的连接数
    pub fn send_to_users(&self, user_ids: &[String], message: &ServerMessage) -> usize {
        let json = message.to_json();
        let mut total_sent = 0;

        for user_id in user_ids {
            if let Some(entry) = self.connections.get(user_id) {
                for conn in entry.iter() {
                    if conn.sender.send(Message::Text(json.clone().into())).is_ok() {
                        total_sent += 1;
                    }
                }
            }
        }

        debug!(
            user_count = user_ids.len(),
            total_sent = total_sent,
            "Message broadcast to users"
        );

        total_sent
    }

    /// 向用户的其他设备发送消息（排除指定设备）
    ///
    /// 用于多设备同步场景
    pub fn send_to_other_devices(
        &self,
        user_id: &str,
        exclude_device_id: &str,
        message: &ServerMessage,
    ) -> usize {
        let json = message.to_json();
        let mut sent_count = 0;

        if let Some(entry) = self.connections.get(user_id) {
            for conn in entry.iter() {
                if conn.device_id != exclude_device_id {
                    if conn.sender.send(Message::Text(json.clone().into())).is_ok() {
                        sent_count += 1;
                    }
                }
            }
        }

        sent_count
    }

    /// 获取用户的所有在线设备 ID
    pub fn get_user_devices(&self, user_id: &str) -> Vec<String> {
        self.connections
            .get(user_id)
            .map(|v| v.iter().map(|c| c.device_id.clone()).collect())
            .unwrap_or_default()
    }

    /// 获取在线用户数
    pub fn online_user_count(&self) -> usize {
        self.online_users.load(Ordering::SeqCst)
    }

    /// 获取总连接数
    pub fn total_connection_count(&self) -> usize {
        self.total_connections.load(Ordering::SeqCst)
    }

    /// 获取所有在线用户 ID
    pub fn get_online_users(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        // ConnectionManager 通常通过 Arc 共享，这里的 clone 创建新实例
        // 实际使用时应该使用 Arc<ConnectionManager>
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_manager() {
        let manager = ConnectionManager::new();

        // 创建测试通道
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        // 注册连接
        manager.register("user1", "device1", tx1);
        assert!(manager.is_online("user1"));
        assert_eq!(manager.online_user_count(), 1);

        // 同一用户第二个设备
        manager.register("user1", "device2", tx2);
        assert_eq!(manager.online_user_count(), 1);
        assert_eq!(manager.total_connection_count(), 2);

        // 获取设备列表
        let devices = manager.get_user_devices("user1");
        assert_eq!(devices.len(), 2);

        // 移除一个设备
        manager.unregister("user1", "device1");
        assert!(manager.is_online("user1"));
        assert_eq!(manager.total_connection_count(), 1);

        // 移除最后一个设备
        manager.unregister("user1", "device2");
        assert!(!manager.is_online("user1"));
        assert_eq!(manager.online_user_count(), 0);
    }
}

