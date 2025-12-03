//! 更新群聊信息处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{UpdateGroupRequest, UpdateJoinModeRequest, SuccessResponse};
use super::state::GroupsState;

/// 更新群聊信息
/// PUT /api/groups/:group_id
pub async fn update_group_info(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.group_service.update_group_info(
        &group_id,
        req.group_name.as_deref(),
        req.group_description.as_deref(),
    ).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("群信息更新成功"))))
}

/// 修改入群模式
/// PUT /api/groups/:group_id/join-mode
pub async fn update_join_mode(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdateJoinModeRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 只有群主可以修改入群模式
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.group_service.update_join_mode(&group_id, &req.join_mode).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("入群模式更新成功"))))
}

