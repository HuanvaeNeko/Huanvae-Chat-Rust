//! 禁言管理处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{MuteMemberRequest, SuccessResponse};
use super::state::GroupsState;

/// 禁言成员
/// POST /api/groups/:group_id/mute
pub async fn mute_member(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<MuteMemberRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 不能禁言自己
    if req.user_id == auth.user_id {
        return Err(AppError::BadRequest("不能禁言自己".to_string()));
    }

    // 获取操作者角色
    let operator_role = state.member_service.get_member_role(&group_id, &auth.user_id).await?
        .ok_or(AppError::Forbidden)?;

    if operator_role.status != "active" || operator_role.role == "member" {
        return Err(AppError::Forbidden);
    }

    // 获取被禁言者角色
    let target_role = state.member_service.get_member_role(&group_id, &req.user_id).await?
        .ok_or_else(|| AppError::BadRequest("该用户不是群成员".to_string()))?;

    if target_role.status != "active" {
        return Err(AppError::BadRequest("该用户不是群成员".to_string()));
    }

    // 群主可以禁言任何人（除了自己），管理员只能禁言普通成员
    if operator_role.role == "admin" && target_role.role != "member" {
        return Err(AppError::BadRequest("管理员只能禁言普通成员".to_string()));
    }

    state.member_service.mute_member(
        &group_id,
        &req.user_id,
        &auth.user_id,
        req.duration_minutes,
        req.reason.as_deref(),
    ).await?;

    let msg = if req.duration_minutes <= 0 {
        "成员已被永久禁言".to_string()
    } else {
        format!("成员已被禁言 {} 分钟", req.duration_minutes)
    };

    Ok(Json(ApiResponse::success(SuccessResponse::new(&msg))))
}

/// 解除禁言
/// DELETE /api/groups/:group_id/mute/:user_id
pub async fn unmute_member(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, user_id)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证操作者是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.member_service.unmute_member(&group_id, &user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("已解除禁言"))))
}

