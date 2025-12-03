//! 删除群消息处理器

use axum::{extract::State, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::group_messages::models::{DeleteGroupMessageRequest, SuccessResponse};
use super::state::GroupMessagesState;

/// 删除群消息（个人删除）
/// DELETE /api/group-messages/delete
pub async fn delete_message(
    State(state): State<GroupMessagesState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<DeleteGroupMessageRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    let message_uuid = Uuid::parse_str(&req.message_uuid)
        .map_err(|_| AppError::BadRequest("无效的消息ID".to_string()))?;

    state.message_service.delete_message(&message_uuid, &auth.user_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("消息已删除"))))
}

