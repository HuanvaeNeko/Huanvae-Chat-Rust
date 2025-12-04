use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::Extension;
use chrono::{DateTime, Utc};

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends_messages::models::{GetMessagesQuery, MessagesListResponse};

use super::state::MessagesState;

/// 获取消息列表处理器
pub async fn get_messages_handler(
    State(state): State<MessagesState>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<impl IntoResponse, AppError> {
    // 1. 验证参数
    if query.friend_id.is_empty() {
        return Err(AppError::BadRequest("friend_id 不能为空".to_string()));
    }

    // 2. 限制 limit 范围
    let limit = query.limit.unwrap_or(50).min(500).max(1);

    // 3. 解析时间戳参数（优先使用 before_time）
    let before_time: Option<DateTime<Utc>> = if let Some(ref time_str) = query.before_time {
        DateTime::parse_from_rfc3339(time_str)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    } else {
        None
    };

    // 4. 调用服务查询消息
    let (messages, has_more) = state
        .service
        .get_messages(&auth.user_id, &query.friend_id, before_time, limit)
        .await?;

    // 5. 返回响应
    Ok((
        StatusCode::OK,
        Json(MessagesListResponse { messages, has_more }),
    ))
}

