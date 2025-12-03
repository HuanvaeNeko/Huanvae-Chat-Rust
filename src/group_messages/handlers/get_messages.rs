//! 获取群消息处理器

use axum::{extract::{Query, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::group_messages::models::{GetGroupMessagesQuery, GroupMessagesResponse};
use super::state::GroupMessagesState;

/// 获取群消息列表
/// GET /api/group-messages?group_id=xxx&before_uuid=xxx&limit=50
pub async fn get_messages(
    State(state): State<GroupMessagesState>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<GetGroupMessagesQuery>,
) -> Result<Json<ApiResponse<GroupMessagesResponse>>, AppError> {
    // 解析群ID
    let group_id = Uuid::parse_str(&query.group_id)
        .map_err(|_| AppError::BadRequest("无效的群ID".to_string()))?;

    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let before_uuid = if let Some(ref uuid_str) = query.before_uuid {
        Some(Uuid::parse_str(uuid_str).map_err(|_| AppError::BadRequest("无效的消息ID".to_string()))?)
    } else {
        None
    };

    let limit = query.limit.unwrap_or(50).min(500);

    let response = state.message_service.get_messages(
        &group_id,
        &auth.user_id,
        before_uuid.as_ref(),
        limit,
    ).await?;

    // 标记已读
    state.message_service.mark_as_read(&group_id, &auth.user_id).await.ok();

    Ok(Json(ApiResponse::success(response)))
}

