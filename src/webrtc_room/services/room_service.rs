//! 房间 CRUD 服务

use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::webrtc_room::models::{CreateRoomRequest, CreateRoomResponse, Room, RoomState};

use super::RoomManager;

/// 房间服务
pub struct RoomService {
    /// 房间管理器
    room_manager: Arc<RoomManager>,
}

impl RoomService {
    /// 创建房间服务
    pub fn new(room_manager: Arc<RoomManager>) -> Self {
        Self { room_manager }
    }

    /// 创建房间
    pub fn create_room(
        &self,
        creator_id: &str,
        request: CreateRoomRequest,
    ) -> CreateRoomResponse {
        // 生成房间 ID (6位大写字母数字)
        let room_id = self.generate_room_id();

        // 生成或使用提供的密码
        let password = request
            .password
            .unwrap_or_else(|| self.generate_password());

        // 密码哈希
        let password_hash = self.hash_password(&password);

        // 房间名称（安全截取，避免越界）
        let name = request.name.unwrap_or_else(|| {
            let display_id: String = creator_id.chars().take(8).collect();
            format!("{}的房间", display_id)
        });

        // 最大参与人数
        let max_participants = request.max_participants.unwrap_or(10).min(50);

        // 过期时间
        let expires_minutes = request.expires_minutes.unwrap_or(120).min(1440) as i64; // 最多24小时
        let now = Utc::now();
        let expires_at = now + Duration::minutes(expires_minutes);

        // 创建房间
        let room = Room {
            room_id: room_id.clone(),
            name: name.clone(),
            creator_id: creator_id.to_string(),
            password_hash,
            max_participants,
            created_at: now,
            expires_at,
        };

        // 注册到管理器
        self.room_manager.create_room(room);

        CreateRoomResponse {
            room_id,
            password,
            name,
            max_participants,
            expires_at,
        }
    }

    /// 验证房间密码
    pub fn verify_password(&self, room_id: &str, password: &str) -> Result<RoomState, RoomError> {
        let room_state = self
            .room_manager
            .get_room(room_id)
            .ok_or(RoomError::NotFound)?;

        // 检查是否过期
        if room_state.room.expires_at < Utc::now() {
            self.room_manager.remove_room(room_id);
            return Err(RoomError::Expired);
        }

        // 验证密码
        let password_hash = self.hash_password(password);
        if room_state.room.password_hash != password_hash {
            return Err(RoomError::InvalidPassword);
        }

        // 检查人数限制
        if room_state.participant_count >= room_state.room.max_participants {
            return Err(RoomError::RoomFull);
        }

        Ok(room_state)
    }

    /// 获取房间信息
    pub fn get_room(&self, room_id: &str) -> Option<RoomState> {
        self.room_manager.get_room(room_id)
    }

    /// 关闭房间
    pub fn close_room(&self, room_id: &str, user_id: &str) -> Result<(), RoomError> {
        let room_state = self
            .room_manager
            .get_room(room_id)
            .ok_or(RoomError::NotFound)?;

        // 只有创建者可以关闭
        if room_state.room.creator_id != user_id {
            return Err(RoomError::NotCreator);
        }

        self.room_manager.remove_room(room_id);
        Ok(())
    }

    /// 生成房间 ID
    fn generate_room_id(&self) -> String {
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // 排除易混淆字符
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect()
    }

    /// 生成随机密码
    fn generate_password(&self) -> String {
        const CHARS: &[u8] = b"0123456789";
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..CHARS.len());
                CHARS[idx] as char
            })
            .collect()
    }

    /// 密码哈希
    fn hash_password(&self, password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 获取房间管理器引用
    pub fn room_manager(&self) -> &Arc<RoomManager> {
        &self.room_manager
    }
}

/// 房间错误
#[derive(Debug, Clone)]
pub enum RoomError {
    /// 房间不存在
    NotFound,
    /// 密码错误
    InvalidPassword,
    /// 房间已过期
    Expired,
    /// 房间已满
    RoomFull,
    /// 不是创建者
    NotCreator,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomError::NotFound => write!(f, "房间不存在"),
            RoomError::InvalidPassword => write!(f, "密码错误"),
            RoomError::Expired => write!(f, "房间已过期"),
            RoomError::RoomFull => write!(f, "房间已满"),
            RoomError::NotCreator => write!(f, "只有创建者可以执行此操作"),
        }
    }
}

