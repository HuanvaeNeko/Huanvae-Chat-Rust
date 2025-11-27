use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::Extension;
use validator::Validate;

use crate::auth::errors::AuthError;
use crate::auth::middleware::AuthContext;
use crate::friends_messages::models::{RecallMessageRequest, SuccessResponse};
use crate::friends_messages::services::MessageService;

use super::state::MessagesState;

/// 撤回消息处理器（2分钟内）
pub async fn recall_message_handler(
    State(state): State<MessagesState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<RecallMessageRequest>,
) -> Result<impl IntoResponse, AuthError> {
    // 1. 验证请求参数
    req.validate()
        .map_err(|e| AuthError::BadRequest(format!("参数验证失败: {}", e)))?;

    // 2. 调用服务撤回消息
    let message_service = MessageService::new(state.db.clone());
    message_service
        .recall_message(&auth.user_id, &req.message_uuid)
        .await?;

    // 3. 返回响应
    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "消息已撤回".to_string(),
        }),
    ))
}

