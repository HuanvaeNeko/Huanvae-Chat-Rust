//! 房间数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 房间信息
#[derive(Debug, Clone, Serialize)]
pub struct Room {
    /// 房间 ID (6位字母数字)
    pub room_id: String,
    /// 房间名称
    pub name: String,
    /// 创建者用户 ID
    pub creator_id: String,
    /// 密码哈希
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// 最大参与人数
    pub max_participants: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 创建房间请求
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    /// 房间名称（可选）
    pub name: Option<String>,
    /// 房间密码（可选，不填则自动生成）
    pub password: Option<String>,
    /// 最大参与人数（可选，默认10）
    pub max_participants: Option<usize>,
    /// 过期时间（分钟，可选，默认120）
    pub expires_minutes: Option<u32>,
}

/// 创建房间响应
#[derive(Debug, Serialize)]
pub struct CreateRoomResponse {
    /// 房间 ID
    pub room_id: String,
    /// 房间密码（明文，仅创建时返回一次）
    pub password: String,
    /// 房间名称
    pub name: String,
    /// 最大参与人数
    pub max_participants: usize,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 加入房间请求
#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    /// 房间密码
    pub password: String,
    /// 显示名称
    pub display_name: String,
}

/// 加入房间响应
#[derive(Debug, Serialize)]
pub struct JoinRoomResponse {
    /// 参与者 ID
    pub participant_id: String,
    /// WebSocket 连接 Token
    pub ws_token: String,
    /// 房间名称
    pub room_name: String,
    /// ICE 服务器配置
    pub ice_servers: Vec<IceServerConfig>,
    /// Token 过期时间
    pub token_expires_at: DateTime<Utc>,
}

/// ICE 服务器配置
#[derive(Debug, Clone, Serialize)]
pub struct IceServerConfig {
    /// 服务器 URLs
    pub urls: Vec<String>,
    /// 用户名（TURN 需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 凭证（TURN 需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

/// 房间状态（用于内部管理）
#[derive(Debug, Clone)]
pub struct RoomState {
    /// 房间基本信息
    pub room: Room,
    /// 当前参与者数量
    pub participant_count: usize,
}

