//! 房间连接管理器
//!
//! 管理房间内的参与者连接和信令转发

use axum::extract::ws::Message;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::webrtc_room::models::{
    Participant, ParticipantConnection, ParticipantInfo, Room, RoomState, ServerSignaling,
};

/// 房间管理器
#[derive(Debug)]
pub struct RoomManager {
    /// 房间信息: room_id -> Room
    rooms: DashMap<String, Room>,
    /// 房间内的参与者: room_id -> (participant_id -> ParticipantConnection)
    participants: DashMap<String, DashMap<String, ParticipantConnection>>,
    /// 总房间数
    total_rooms: AtomicUsize,
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomManager {
    /// 创建房间管理器
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            participants: DashMap::new(),
            total_rooms: AtomicUsize::new(0),
        }
    }

    /// 创建房间
    pub fn create_room(&self, room: Room) {
        let room_id = room.room_id.clone();
        self.rooms.insert(room_id.clone(), room);
        self.participants.insert(room_id.clone(), DashMap::new());
        self.total_rooms.fetch_add(1, Ordering::SeqCst);
        info!(room_id = %room_id, "房间已创建");
    }

    /// 获取房间信息
    pub fn get_room(&self, room_id: &str) -> Option<RoomState> {
        let room = self.rooms.get(room_id)?.clone();
        let participant_count = self
            .participants
            .get(room_id)
            .map(|p| p.len())
            .unwrap_or(0);

        Some(RoomState {
            room,
            participant_count,
        })
    }

    /// 移除房间
    pub fn remove_room(&self, room_id: &str) {
        // 通知所有参与者房间关闭
        if let Some((_, participants)) = self.participants.remove(room_id) {
            let close_msg = ServerSignaling::RoomClosed {
                reason: "房间已关闭".to_string(),
            };
            let json = close_msg.to_json();
            for entry in participants.iter() {
                let _ = entry.value().sender.send(Message::Text(json.clone().into()));
            }
        }

        if self.rooms.remove(room_id).is_some() {
            self.total_rooms.fetch_sub(1, Ordering::SeqCst);
            info!(room_id = %room_id, "房间已关闭");
        }
    }

    /// 添加参与者到房间
    pub fn add_participant(
        &self,
        room_id: &str,
        participant: Participant,
        sender: mpsc::UnboundedSender<Message>,
    ) -> Option<Vec<ParticipantInfo>> {
        let room_participants = self.participants.get(room_id)?;

        // 获取当前参与者列表
        let current_participants: Vec<ParticipantInfo> = room_participants
            .iter()
            .map(|entry| ParticipantInfo::from(&entry.value().participant))
            .collect();

        let participant_id = participant.participant_id.clone();
        let participant_info = ParticipantInfo::from(&participant);

        // 添加新参与者
        room_participants.insert(
            participant_id.clone(),
            ParticipantConnection {
                participant,
                sender,
            },
        );

        // 通知其他参与者有新人加入
        let join_msg = ServerSignaling::PeerJoined {
            participant: participant_info,
        };
        self.broadcast_to_room_except(room_id, &participant_id, &join_msg);

        debug!(room_id = %room_id, participant_id = %participant_id, "参与者已加入");

        Some(current_participants)
    }

    /// 移除参与者
    pub fn remove_participant(&self, room_id: &str, participant_id: &str) {
        if let Some(room_participants) = self.participants.get(room_id) {
            if room_participants.remove(participant_id).is_some() {
                // 通知其他参与者
                let leave_msg = ServerSignaling::PeerLeft {
                    participant_id: participant_id.to_string(),
                };
                self.broadcast_to_room(room_id, &leave_msg);

                debug!(room_id = %room_id, participant_id = %participant_id, "参与者已离开");

                // 如果房间空了，检查是否需要清理
                if room_participants.is_empty() {
                    // 可以选择立即删除或等待过期
                    debug!(room_id = %room_id, "房间已空");
                }
            }
        }
    }

    /// 获取房间内的参与者列表
    pub fn get_participants(&self, room_id: &str) -> Vec<ParticipantInfo> {
        self.participants
            .get(room_id)
            .map(|p| {
                p.iter()
                    .map(|entry| ParticipantInfo::from(&entry.value().participant))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 向房间内单个参与者发送消息
    pub fn send_to_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        message: &ServerSignaling,
    ) -> bool {
        if let Some(room_participants) = self.participants.get(room_id) {
            if let Some(conn) = room_participants.get(participant_id) {
                let json = message.to_json();
                return conn.sender.send(Message::Text(json.into())).is_ok();
            }
        }
        false
    }

    /// 广播消息到房间所有参与者
    pub fn broadcast_to_room(&self, room_id: &str, message: &ServerSignaling) {
        if let Some(room_participants) = self.participants.get(room_id) {
            let json = message.to_json();
            for entry in room_participants.iter() {
                let _ = entry.value().sender.send(Message::Text(json.clone().into()));
            }
        }
    }

    /// 广播消息到房间（排除某个参与者）
    pub fn broadcast_to_room_except(
        &self,
        room_id: &str,
        exclude_id: &str,
        message: &ServerSignaling,
    ) {
        if let Some(room_participants) = self.participants.get(room_id) {
            let json = message.to_json();
            for entry in room_participants.iter() {
                if entry.key() != exclude_id {
                    let _ = entry.value().sender.send(Message::Text(json.clone().into()));
                }
            }
        }
    }

    /// 转发信令消息
    pub fn forward_signaling(
        &self,
        room_id: &str,
        from_id: &str,
        to_id: &str,
        message: ServerSignaling,
    ) -> bool {
        if let Some(room_participants) = self.participants.get(room_id) {
            if let Some(conn) = room_participants.get(to_id) {
                let json = message.to_json();
                if conn.sender.send(Message::Text(json.into())).is_ok() {
                    debug!(room_id = %room_id, from = %from_id, to = %to_id, "信令已转发");
                    return true;
                }
            }
        }
        warn!(room_id = %room_id, from = %from_id, to = %to_id, "信令转发失败");
        false
    }

    /// 获取总房间数
    pub fn total_rooms(&self) -> usize {
        self.total_rooms.load(Ordering::SeqCst)
    }

    /// 清理过期房间
    pub fn cleanup_expired_rooms(&self) {
        let now = Utc::now();
        let expired: Vec<String> = self
            .rooms
            .iter()
            .filter(|entry| entry.value().expires_at < now)
            .map(|entry| entry.key().clone())
            .collect();

        for room_id in expired {
            self.remove_room(&room_id);
            info!(room_id = %room_id, "过期房间已清理");
        }
    }
}

