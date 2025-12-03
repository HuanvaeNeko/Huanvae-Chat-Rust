//! 成员管理处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::*;
use super::state::GroupsState;

/// 获取群成员列表
/// GET /api/groups/:group_id/members
pub async fn get_members(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MemberListResponse>>, AppError> {
    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let members = state.member_service.get_members(&group_id).await?;
    let total = members.len() as i32;

    Ok(Json(ApiResponse::success(MemberListResponse { members, total })))
}

/// 邀请成员
/// POST /api/groups/:group_id/invite
pub async fn invite_members(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<Json<ApiResponse<InviteMemberResponse>>, AppError> {
    // 验证群是否存在且活跃
    if !state.group_service.verify_group_active(&group_id).await? {
        return Err(AppError::BadRequest("群聊不存在或已解散".to_string()));
    }

    // 获取邀请人角色
    let inviter_role = state.member_service.get_member_role(&group_id, &auth.user_id).await?
        .ok_or(AppError::Forbidden)?;

    if inviter_role.status != "active" {
        return Err(AppError::Forbidden);
    }

    // 获取群的入群模式
    let group_info = state.group_service.get_group_info(&group_id).await?;
    
    // 检查入群模式是否允许邀请
    if group_info.join_mode == "closed" {
        return Err(AppError::BadRequest("群聊已关闭入群".to_string()));
    }
    if group_info.join_mode == "admin_invite_only" && inviter_role.role == "member" {
        return Err(AppError::BadRequest("只有群主或管理员可以邀请成员".to_string()));
    }

    let mut results = Vec::new();
    let is_admin_or_owner = inviter_role.role == "owner" || inviter_role.role == "admin";

    for user_id in &req.user_ids {
        // 检查用户是否存在
        let user_exists: Option<(String,)> = sqlx::query_as(
            r#"SELECT "user-id" FROM "users" WHERE "user-id" = $1"#
        )
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

        if user_exists.is_none() {
            results.push(InviteResult {
                user_id: user_id.clone(),
                success: false,
                message: "用户不存在".to_string(),
            });
            continue;
        }

        // 检查是否已在群中
        if state.member_service.verify_active_member(&group_id, user_id).await? {
            results.push(InviteResult {
                user_id: user_id.clone(),
                success: false,
                message: "用户已是群成员".to_string(),
            });
            continue;
        }

        // 检查是否已有待处理的邀请
        let existing_request: Option<(Uuid,)> = sqlx::query_as(
            r#"SELECT "id" FROM "group-join-requests" 
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'pending'"#
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

        if existing_request.is_some() {
            results.push(InviteResult {
                user_id: user_id.clone(),
                success: false,
                message: "已有待处理的邀请".to_string(),
            });
            continue;
        }

        // 创建邀请记录
        let request_type = if inviter_role.role == "owner" {
            "owner_invite"
        } else if inviter_role.role == "admin" {
            "admin_invite"
        } else {
            "member_invite"
        };

        let request_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        sqlx::query(
            r#"INSERT INTO "group-join-requests"
               ("id", "group-id", "user-id", "request-type", "inviter-id", "message", "status", "created-at", "expires-at")
               VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)"#
        )
        .bind(request_id)
        .bind(group_id)
        .bind(user_id)
        .bind(request_type)
        .bind(&auth.user_id)
        .bind(&req.message)
        .bind(now)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("创建邀请记录失败: {}", e);
            AppError::Internal
        })?;

        results.push(InviteResult {
            user_id: user_id.clone(),
            success: true,
            message: if is_admin_or_owner {
                "邀请已发送，待对方同意".to_string()
            } else {
                "邀请已发送，待对方同意并经管理员审核".to_string()
            },
        });
    }

    Ok(Json(ApiResponse::success(InviteMemberResponse { results })))
}

/// 退出群聊
/// POST /api/groups/:group_id/leave
pub async fn leave_group(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<LeaveGroupRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 检查用户是否是群主
    if state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::BadRequest("群主不能退出群聊，请先转让群主或解散群聊".to_string()));
    }

    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::BadRequest("您不是该群成员".to_string()));
    }

    state.member_service.leave_group(&group_id, &auth.user_id, req.reason.as_deref()).await?;
    state.group_service.decrement_member_count(&group_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("已退出群聊"))))
}

/// 移除成员
/// DELETE /api/groups/:group_id/members/:user_id
pub async fn remove_member(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, user_id)): Path<(Uuid, String)>,
    Json(req): Json<RemoveMemberRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 不能移除自己
    if user_id == auth.user_id {
        return Err(AppError::BadRequest("不能移除自己".to_string()));
    }

    // 获取操作者角色
    let operator_role = state.member_service.get_member_role(&group_id, &auth.user_id).await?
        .ok_or(AppError::Forbidden)?;

    if operator_role.status != "active" || operator_role.role == "member" {
        return Err(AppError::Forbidden);
    }

    // 获取被移除者角色
    let target_role = state.member_service.get_member_role(&group_id, &user_id).await?
        .ok_or_else(|| AppError::BadRequest("该用户不是群成员".to_string()))?;

    if target_role.status != "active" {
        return Err(AppError::BadRequest("该用户不是群成员".to_string()));
    }

    // 群主可以踢任何人，管理员只能踢普通成员
    if operator_role.role == "admin" && target_role.role != "member" {
        return Err(AppError::BadRequest("管理员只能移除普通成员".to_string()));
    }

    state.member_service.remove_member(&group_id, &user_id, &auth.user_id, req.reason.as_deref()).await?;
    state.group_service.decrement_member_count(&group_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("成员已被移除"))))
}

