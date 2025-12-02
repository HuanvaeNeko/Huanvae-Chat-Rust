use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, PendingRequestDto};
use crate::auth::middleware::extract_auth_context;
use crate::common::AppError;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_pending_requests_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<PendingRequestDto>>, AppError> {
    let auth = extract_auth_context(&request)?;
    
    // 查询待处理的好友请求（别人发给我的）
    let requests: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT "id", "from-user-id", "message", "created-at"
           FROM "friend-requests"
           WHERE "to-user-id" = $1 AND "status" = 'pending'
           ORDER BY "created-at" DESC"#,
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    let items = requests
        .into_iter()
        .map(|(id, from_user_id, message, created_at)| PendingRequestDto {
            request_id: id.to_string(),
            request_user_id: from_user_id,
            request_message: if message.is_empty() { None } else { Some(message) },
            request_time: created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ListResponse { items }))
}
