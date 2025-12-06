//! WebRTC 房间模块
//!
//! 提供 WebRTC 实时音视频通信的房间管理和信令服务
//!
//! ## 功能
//!
//! - 房间创建（需登录）
//! - 房间加入（无需登录，密码验证）
//! - 信令 WebSocket（SDP、ICE Candidate 转发）
//! - TURN 服务器自动分配
//!
//! ## API
//!
//! - `POST /api/webrtc/rooms` - 创建房间（需登录）
//! - `POST /api/webrtc/rooms/{room_id}/join` - 加入房间（无需登录）
//! - `GET /ws/webrtc/rooms/{room_id}?token=xxx` - 信令 WebSocket

pub mod handlers;
pub mod models;
pub mod services;

pub use handlers::{webrtc_room_routes, WebRTCState};
pub use services::{RoomManager, RoomService, RoomTokenService};

