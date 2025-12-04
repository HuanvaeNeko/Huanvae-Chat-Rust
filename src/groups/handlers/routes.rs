//! 群聊路由配置

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use crate::auth::middleware::{auth_guard, AuthState};
use super::state::GroupsState;
use super::{
    create_group::create_group,
    get_group::{get_group_info, get_my_groups, search_groups},
    update_group::{update_group_info, update_join_mode, upload_group_avatar, update_member_nickname},
    disband_group::disband_group,
    members::{get_members, invite_members, leave_group, remove_member},
    roles::{transfer_owner, set_admin, remove_admin},
    mute::{mute_member, unmute_member},
    invite_codes::{create_invite_code, get_invite_codes, revoke_invite_code, join_by_code},
    join_requests::{apply_join, get_pending_requests, approve_request, reject_request, get_invitations, accept_invitation, decline_invitation},
    notices::{publish_notice, get_notices, update_notice, delete_notice},
};

/// 创建群聊路由
pub fn create_group_routes(state: GroupsState, auth_state: AuthState) -> Router {
    Router::new()
        // 群聊基础操作
        .route("/", post(create_group))
        .route("/my", get(get_my_groups))
        .route("/search", get(search_groups))
        .route("/join_by_code", post(join_by_code))
        .route("/invitations", get(get_invitations))
        .route("/invitations/{request_id}/accept", post(accept_invitation))
        .route("/invitations/{request_id}/decline", post(decline_invitation))
        
        // 群聊详情
        .route("/{group_id}", get(get_group_info))
        .route("/{group_id}", put(update_group_info))
        .route("/{group_id}", delete(disband_group))
        .route("/{group_id}/join_mode", put(update_join_mode))
        .route("/{group_id}/avatar", post(upload_group_avatar))
        .route("/{group_id}/nickname", put(update_member_nickname))
        
        // 成员管理
        .route("/{group_id}/members", get(get_members))
        .route("/{group_id}/invite", post(invite_members))
        .route("/{group_id}/leave", post(leave_group))
        .route("/{group_id}/members/{user_id}", delete(remove_member))
        
        // 角色管理
        .route("/{group_id}/transfer", post(transfer_owner))
        .route("/{group_id}/admins", post(set_admin))
        .route("/{group_id}/admins/{user_id}", delete(remove_admin))
        
        // 禁言管理
        .route("/{group_id}/mute", post(mute_member))
        .route("/{group_id}/mute/{user_id}", delete(unmute_member))
        
        // 邀请码管理
        .route("/{group_id}/invite_codes", post(create_invite_code))
        .route("/{group_id}/invite_codes", get(get_invite_codes))
        .route("/{group_id}/invite_codes/{code_id}", delete(revoke_invite_code))
        
        // 入群申请
        .route("/{group_id}/apply", post(apply_join))
        .route("/{group_id}/requests", get(get_pending_requests))
        .route("/{group_id}/requests/{request_id}/approve", post(approve_request))
        .route("/{group_id}/requests/{request_id}/reject", post(reject_request))
        
        // 群公告
        .route("/{group_id}/notices", post(publish_notice))
        .route("/{group_id}/notices", get(get_notices))
        .route("/{group_id}/notices/{notice_id}", put(update_notice))
        .route("/{group_id}/notices/{notice_id}", delete(delete_notice))
        
        // 应用状态和认证中间件
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth_state, auth_guard))
}
