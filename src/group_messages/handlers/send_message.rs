//! 发送群消息处理器

use axum::{extract::State, Extension, Json};
use chrono::Utc;
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::group_messages::models::{SendGroupMessageRequest, SendMessageResponse};
use super::state::GroupMessagesState;

/// 发送群消息
/// POST /api/group-messages
pub async fn send_message(
    State(state): State<GroupMessagesState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SendGroupMessageRequest>,
) -> Result<Json<ApiResponse<SendMessageResponse>>, AppError> {
    // 解析群ID
    let group_id = Uuid::parse_str(&req.group_id)
        .map_err(|_| AppError::BadRequest("无效的群ID".to_string()))?;

    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    // 检查是否被禁言
    if state.member_service.is_muted(&group_id, &auth.user_id).await? {
        return Err(AppError::BadRequest("您已被禁言，无法发送消息".to_string()));
    }

    // 验证消息内容
    if req.message_content.trim().is_empty() {
        return Err(AppError::BadRequest("消息内容不能为空".to_string()));
    }

    let message_type = req.message_type.as_deref().unwrap_or("text");
    let reply_to = if let Some(ref uuid_str) = req.reply_to {
        Some(Uuid::parse_str(uuid_str).map_err(|_| AppError::BadRequest("无效的回复消息ID".to_string()))?)
    } else {
        None
    };

    let response = state.message_service.send_message(
        &group_id,
        &auth.user_id,
        &req.message_content,
        message_type,
        req.file_uuid.as_deref(),
        req.file_url.as_deref(),
        req.file_size,
        reply_to.as_ref(),
    ).await?;

    // 发送 WebSocket 实时通知
    if let Some(ref notification_service) = state.notification_service {
        // 获取发送者昵称和群名称
        let sender_info: Option<(Option<String>,)> = sqlx::query_as(
            r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#,
        )
        .bind(&auth.user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        let group_info: Option<(Option<String>,)> = sqlx::query_as(
            r#"SELECT "group-name" FROM "groups" WHERE "group-id" = $1"#,
        )
        .bind(&group_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        let sender_nickname = sender_info
            .and_then(|(n,)| n)
            .unwrap_or_else(|| auth.user_id.clone());
        let group_name = group_info
            .and_then(|(n,)| n)
            .unwrap_or_else(|| "群聊".to_string());

        let message_uuid = Uuid::parse_str(&response.message_uuid).ok();
        let send_time_dt = chrono::DateTime::parse_from_rfc3339(&response.send_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        if let Some(msg_uuid) = message_uuid {
            if let Err(e) = notification_service
                .notify_group_message(
                    &group_id,
                    &group_name,
                    &auth.user_id,
                    &sender_nickname,
                    &msg_uuid,
                    &req.message_content,
                    message_type,
                    send_time_dt,
                )
                .await
            {
                tracing::warn!("发送群消息通知失败: {}", e);
            }
        }
    }

    Ok(Json(ApiResponse::success(response)))
}

