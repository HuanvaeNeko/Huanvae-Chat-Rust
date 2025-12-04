//! 撤回群消息处理器

use axum::{extract::State, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::group_messages::models::{RecallGroupMessageRequest, SuccessResponse};
use super::state::GroupMessagesState;

/// 撤回群消息
/// POST /api/group-messages/recall
pub async fn recall_message(
    State(state): State<GroupMessagesState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<RecallGroupMessageRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    let message_uuid = Uuid::parse_str(&req.message_uuid)
        .map_err(|_| AppError::BadRequest("无效的消息ID".to_string()))?;

    // 查询消息所属群
    let group_id: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "group-id" FROM "group-messages" WHERE "message-uuid" = $1"#,
    )
    .bind(message_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

    let (group_id,) = group_id.ok_or_else(|| AppError::BadRequest("消息不存在".to_string()))?;

    // 检查用户是否是群主或管理员
    let is_admin_or_owner = state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await?;

    state.message_service.recall_message(&message_uuid, &auth.user_id, is_admin_or_owner).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("消息已撤回"))))
}

