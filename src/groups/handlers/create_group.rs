//! 创建群聊处理器

use axum::{extract::State, Extension, Json};
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{CreateGroupRequest, CreateGroupResponse};
use super::state::GroupsState;

/// 创建群聊
/// POST /api/groups
pub async fn create_group(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<ApiResponse<CreateGroupResponse>>, AppError> {
    // 验证群名称
    if req.group_name.trim().is_empty() {
        return Err(AppError::BadRequest("群名称不能为空".to_string()));
    }

    let response = state.group_service.create_group(
        &auth.user_id,
        &req.group_name,
        req.group_description.as_deref(),
        req.join_mode.as_deref(),
    ).await?;

    Ok(Json(ApiResponse::success(response)))
}

