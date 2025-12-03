//! 邀请码管理处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::*;
use super::state::GroupsState;

/// 生成邀请码
/// POST /api/groups/:group_id/invite-codes
pub async fn create_invite_code(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<CreateInviteCodeRequest>,
) -> Result<Json<ApiResponse<CreateInviteCodeResponse>>, AppError> {
    // 验证群是否存在且活跃
    if !state.group_service.verify_group_active(&group_id).await? {
        return Err(AppError::BadRequest("群聊不存在或已解散".to_string()));
    }

    // 获取用户角色
    let member_role = state.member_service.get_member_role(&group_id, &auth.user_id).await?
        .ok_or(AppError::Forbidden)?;

    if member_role.status != "active" {
        return Err(AppError::Forbidden);
    }

    // 检查入群模式
    let group_info = state.group_service.get_group_info(&group_id).await?;
    if group_info.join_mode == "closed" {
        return Err(AppError::BadRequest("群聊已关闭入群".to_string()));
    }
    if group_info.join_mode == "admin_invite_only" && member_role.role == "member" {
        return Err(AppError::BadRequest("只有群主或管理员可以生成邀请码".to_string()));
    }

    let response = state.invite_code_service.create_invite_code(
        &group_id,
        &auth.user_id,
        &member_role.role,
        req.max_uses,
        req.expires_in_hours,
    ).await?;

    Ok(Json(ApiResponse::success(response)))
}

/// 获取邀请码列表
/// GET /api/groups/:group_id/invite-codes
pub async fn get_invite_codes(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<InviteCodeListResponse>>, AppError> {
    // 获取用户角色
    let member_role = state.member_service.get_member_role(&group_id, &auth.user_id).await?
        .ok_or(AppError::Forbidden)?;

    if member_role.status != "active" {
        return Err(AppError::Forbidden);
    }

    let is_admin = member_role.role == "owner" || member_role.role == "admin";
    let codes = state.invite_code_service.get_invite_codes(&group_id, &auth.user_id, is_admin).await?;

    Ok(Json(ApiResponse::success(InviteCodeListResponse { codes })))
}

/// 撤销邀请码
/// DELETE /api/groups/:group_id/invite-codes/:code_id
pub async fn revoke_invite_code(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, code_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.invite_code_service.revoke_invite_code(&code_id, &auth.user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("邀请码已撤销"))))
}

/// 通过邀请码入群
/// POST /api/groups/join-by-code
pub async fn join_by_code(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<JoinByCodeRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证并使用邀请码
    let (group_id, invite_code) = state.invite_code_service.validate_and_use_code(&req.code).await?;

    // 验证群是否存在且活跃
    if !state.group_service.verify_group_active(&group_id).await? {
        return Err(AppError::BadRequest("群聊不存在或已解散".to_string()));
    }

    // 检查用户是否已在群中
    if state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::BadRequest("您已是该群成员".to_string()));
    }

    // 根据邀请码类型处理
    if invite_code.code_type == "direct" {
        // 直通码，直接入群
        state.member_service.add_member(
            &group_id,
            &auth.user_id,
            "direct_invite_code",
            Some(&invite_code.creator_id),
            None,
            Some(&invite_code.id),
        ).await?;
        state.group_service.increment_member_count(&group_id).await?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("已成功加入群聊"))))
    } else {
        // 普通码，创建待审核申请
        let request_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        sqlx::query(
            r#"INSERT INTO "group-join-requests"
               ("id", "group-id", "user-id", "request-type", "inviter-id", "invite-code-id", "status", "created-at", "expires-at")
               VALUES ($1, $2, $3, 'normal_invite_code', $4, $5, 'pending', $6, $7)"#
        )
        .bind(request_id)
        .bind(group_id)
        .bind(&auth.user_id)
        .bind(&invite_code.creator_id)
        .bind(invite_code.id)
        .bind(now)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("创建入群申请失败: {}", e);
            AppError::Internal
        })?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("申请已提交，等待管理员审核"))))
    }
}

