//! 群聊响应模型

use serde::{Deserialize, Serialize};

/// 创建群聊响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroupResponse {
    pub group_id: String,
    pub group_name: String,
    pub created_at: String,
}

/// 群列表项
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupListItem {
    pub group_id: String,
    pub group_name: String,
    pub group_avatar_url: Option<String>,
    pub role: String,
    pub unread_count: Option<i32>,
    pub last_message_content: Option<String>,
    pub last_message_time: Option<String>,
}

/// 群成员列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub members: Vec<super::MemberInfo>,
    pub total: i32,
}

/// 邀请结果
#[derive(Debug, Serialize, Deserialize)]
pub struct InviteResult {
    pub user_id: String,
    pub success: bool,
    pub message: String,
}

/// 邀请成员响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InviteMemberResponse {
    pub results: Vec<InviteResult>,
}

/// 入群申请信息
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequestInfo {
    pub request_id: String,
    pub group_id: String,
    pub group_name: Option<String>,
    pub user_id: String,
    pub user_nickname: Option<String>,
    pub request_type: String,
    pub inviter_id: Option<String>,
    pub inviter_nickname: Option<String>,
    pub message: Option<String>,
    pub user_accepted: bool,
    pub status: String,
    pub created_at: String,
}

/// 入群申请列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequestListResponse {
    pub requests: Vec<JoinRequestInfo>,
}

/// 收到的邀请信息
#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationInfo {
    pub request_id: String,
    pub group_id: String,
    pub group_name: String,
    pub group_avatar_url: Option<String>,
    pub inviter_id: String,
    pub inviter_nickname: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

/// 收到的邀请列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationListResponse {
    pub invitations: Vec<InvitationInfo>,
}

/// 操作成功响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
        }
    }
}

