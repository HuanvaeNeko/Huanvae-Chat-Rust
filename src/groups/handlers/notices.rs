//! 群公告处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::*;
use super::state::GroupsState;

/// 发布公告
/// POST /api/groups/:group_id/notices
pub async fn publish_notice(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<PublishNoticeRequest>,
) -> Result<Json<ApiResponse<PublishNoticeResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    if req.content.trim().is_empty() {
        return Err(AppError::BadRequest("公告内容不能为空".to_string()));
    }

    let response = state.notice_service.publish_notice(
        &group_id,
        &auth.user_id,
        req.title.as_deref(),
        &req.content,
        req.is_pinned.unwrap_or(false),
    ).await?;

    Ok(Json(ApiResponse::success(response)))
}

/// 获取公告列表
/// GET /api/groups/:group_id/notices
pub async fn get_notices(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<NoticeListResponse>>, AppError> {
    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let notices = state.notice_service.get_notices(&group_id).await?;

    Ok(Json(ApiResponse::success(NoticeListResponse { notices })))
}

/// 更新公告
/// PUT /api/groups/:group_id/notices/:notice_id
pub async fn update_notice(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, notice_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateNoticeRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.notice_service.update_notice(
        &notice_id,
        req.title.as_deref(),
        req.content.as_deref(),
        req.is_pinned,
    ).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("公告更新成功"))))
}

/// 删除公告
/// DELETE /api/groups/:group_id/notices/:notice_id
pub async fn delete_notice(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, notice_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.notice_service.delete_notice(&notice_id, &auth.user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("公告已删除"))))
}

