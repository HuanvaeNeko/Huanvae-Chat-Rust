//! 信令消息定义

use serde::{Deserialize, Serialize};

use super::ParticipantInfo;

// ========================================
// 客户端 → 服务器消息
// ========================================

/// 客户端发送的信令消息
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientSignaling {
    /// SDP Offer
    Offer {
        /// 目标参与者 ID
        to: String,
        /// SDP 内容
        sdp: String,
    },

    /// SDP Answer
    Answer {
        /// 目标参与者 ID
        to: String,
        /// SDP 内容
        sdp: String,
    },

    /// ICE Candidate
    Candidate {
        /// 目标参与者 ID
        to: String,
        /// ICE Candidate 信息
        candidate: IceCandidate,
    },

    /// 离开房间
    Leave,
}

/// ICE Candidate 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// Candidate 字符串
    pub candidate: String,
    /// SDP M-Line Index
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_m_line_index: Option<u32>,
    /// SDP Mid
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
}

// ========================================
// 服务器 → 客户端消息
// ========================================

/// 服务器发送的信令消息
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerSignaling {
    /// 加入成功
    Joined {
        /// 自己的参与者 ID
        participant_id: String,
        /// 当前房间内所有参与者
        participants: Vec<ParticipantInfo>,
    },

    /// 新参与者加入
    PeerJoined {
        /// 新参与者信息
        participant: ParticipantInfo,
    },

    /// 参与者离开
    PeerLeft {
        /// 离开的参与者 ID
        participant_id: String,
    },

    /// 收到 SDP Offer
    Offer {
        /// 发送者 ID
        from: String,
        /// SDP 内容
        sdp: String,
    },

    /// 收到 SDP Answer
    Answer {
        /// 发送者 ID
        from: String,
        /// SDP 内容
        sdp: String,
    },

    /// 收到 ICE Candidate
    Candidate {
        /// 发送者 ID
        from: String,
        /// ICE Candidate 信息
        candidate: IceCandidate,
    },

    /// 房间关闭
    RoomClosed {
        /// 关闭原因
        reason: String,
    },

    /// 错误
    Error {
        /// 错误码
        code: String,
        /// 错误信息
        message: String,
    },
}

impl ServerSignaling {
    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","code":"serialize_error","message":"Failed to serialize"}"#
                .to_string()
        })
    }
}

impl ClientSignaling {
    /// 从 JSON 解析
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

