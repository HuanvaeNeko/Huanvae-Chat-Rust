//! 解散群聊处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::SuccessResponse;
use super::state::GroupsState;

/// 解散群聊
/// DELETE /api/groups/:group_id
pub async fn disband_group(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 只有群主可以解散群聊
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.group_service.disband_group(&group_id, &auth.user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("群聊已解散"))))
}

