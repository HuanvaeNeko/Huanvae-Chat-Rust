//! 群聊请求模型

use serde::{Deserialize, Serialize};

/// 创建群聊请求
#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub group_name: String,
    pub group_description: Option<String>,
    pub join_mode: Option<String>,
}

/// 更新群聊信息请求
#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub group_name: Option<String>,
    pub group_description: Option<String>,
}

/// 修改入群模式请求
#[derive(Debug, Deserialize)]
pub struct UpdateJoinModeRequest {
    pub join_mode: String,
}

/// 邀请成员请求
#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub user_ids: Vec<String>,
    pub message: Option<String>,
}

/// 退出群聊请求
#[derive(Debug, Deserialize)]
pub struct LeaveGroupRequest {
    pub reason: Option<String>,
}

/// 移除成员请求
#[derive(Debug, Deserialize)]
pub struct RemoveMemberRequest {
    pub reason: Option<String>,
}

/// 转让群主请求
#[derive(Debug, Deserialize)]
pub struct TransferOwnerRequest {
    pub new_owner_id: String,
}

/// 设置管理员请求
#[derive(Debug, Deserialize)]
pub struct SetAdminRequest {
    pub user_id: String,
}

/// 禁言成员请求
#[derive(Debug, Deserialize)]
pub struct MuteMemberRequest {
    pub user_id: String,
    pub duration_minutes: i64,  // 禁言时长（分钟），0表示永久
    pub reason: Option<String>,
}

/// 解除禁言请求
#[derive(Debug, Deserialize)]
pub struct UnmuteMemberRequest {
    pub user_id: String,
}

/// 创建邀请码请求
#[derive(Debug, Deserialize)]
pub struct CreateInviteCodeRequest {
    pub max_uses: Option<i32>,
    pub expires_in_hours: Option<i32>,
}

/// 通过邀请码入群请求
#[derive(Debug, Deserialize)]
pub struct JoinByCodeRequest {
    pub code: String,
}

/// 申请入群请求
#[derive(Debug, Deserialize)]
pub struct ApplyJoinRequest {
    pub message: Option<String>,
}

/// 处理入群申请请求
#[derive(Debug, Deserialize)]
pub struct ProcessJoinRequestBody {
    pub reason: Option<String>,
}

/// 发布公告请求
#[derive(Debug, Deserialize)]
pub struct PublishNoticeRequest {
    pub title: Option<String>,
    pub content: String,
    pub is_pinned: Option<bool>,
}

/// 更新公告请求
#[derive(Debug, Deserialize)]
pub struct UpdateNoticeRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub is_pinned: Option<bool>,
}

/// 接受/拒绝邀请请求
#[derive(Debug, Serialize, Deserialize)]
pub struct InvitationActionRequest {
    pub action: String,  // "accept" or "decline"
}

