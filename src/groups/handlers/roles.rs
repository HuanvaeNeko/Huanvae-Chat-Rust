//! 角色管理处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{TransferOwnerRequest, SetAdminRequest, SuccessResponse};
use super::state::GroupsState;

/// 转让群主
/// POST /api/groups/:group_id/transfer
pub async fn transfer_owner(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<TransferOwnerRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证当前用户是群主
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    // 不能转让给自己
    if req.new_owner_id == auth.user_id {
        return Err(AppError::BadRequest("不能转让给自己".to_string()));
    }

    // 验证新群主是群成员
    if !state.member_service.verify_active_member(&group_id, &req.new_owner_id).await? {
        return Err(AppError::BadRequest("目标用户不是群成员".to_string()));
    }

    // 执行转让
    state.member_service.transfer_owner(&group_id, &auth.user_id, &req.new_owner_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("群主已转让"))))
}

/// 设置管理员
/// POST /api/groups/:group_id/admins
pub async fn set_admin(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<SetAdminRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 只有群主可以设置管理员
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    // 不能设置自己
    if req.user_id == auth.user_id {
        return Err(AppError::BadRequest("不能设置自己为管理员".to_string()));
    }

    state.member_service.set_admin(&group_id, &req.user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("已设置为管理员"))))
}

/// 取消管理员
/// DELETE /api/groups/:group_id/admins/:user_id
pub async fn remove_admin(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, user_id)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 只有群主可以取消管理员
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.member_service.remove_admin(&group_id, &user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("已取消管理员"))))
}

