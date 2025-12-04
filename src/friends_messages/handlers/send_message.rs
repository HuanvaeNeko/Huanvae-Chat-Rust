use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::Extension;
use chrono::Utc;
use validator::Validate;

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends_messages::models::{SendMessageRequest, SendMessageResponse};

use super::state::MessagesState;

/// 发送消息处理器
pub async fn send_message_handler(
    State(state): State<MessagesState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. 验证请求参数
    req.validate()
        .map_err(|e| AppError::BadRequest(format!("参数验证失败: {}", e)))?;

    // 2. 验证消息类型
    let valid_types = ["text", "image", "video", "file"];
    if !valid_types.contains(&req.message_type.as_str()) {
        return Err(AppError::BadRequest(
            "消息类型必须是: text, image, video, file".to_string(),
        ));
    }

    // 3. 调用服务发送消息
    let (message_uuid, send_time) = state
        .service
        .send_message(
            &auth.user_id,
            &req.receiver_id,
            &req.message_content,
            &req.message_type,
            req.file_uuid,
            req.file_url,
            req.file_size,
        )
        .await?;

    // 4. 发送 WebSocket 实时通知
    if let Some(ref notification_service) = state.notification_service {
        // 获取发送者昵称
        let sender_nickname: Option<String> = sqlx::query_scalar(
            r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#,
        )
        .bind(&auth.user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        let nickname = sender_nickname.unwrap_or_else(|| auth.user_id.clone());
        let send_time_dt = chrono::DateTime::parse_from_rfc3339(&send_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        if let Err(e) = notification_service
            .notify_friend_message(
                &auth.user_id,
                &nickname,
                &req.receiver_id,
                &message_uuid,
                &req.message_content,
                &req.message_type,
                send_time_dt,
            )
            .await
        {
            tracing::warn!("发送消息通知失败: {}", e);
        }
    }

    // 5. 返回响应
    Ok((
        StatusCode::OK,
        Json(SendMessageResponse {
            message_uuid,
            send_time,
        }),
    ))
}

