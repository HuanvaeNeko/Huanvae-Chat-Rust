use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, PendingRequestDto};
use crate::auth::{errors::AuthError, middleware::extract_auth_context};
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_pending_requests_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<PendingRequestDto>>, AuthError> {
    let auth = extract_auth_context(&request)?;
    
    // 查询待处理的好友请求（别人发给我的）
    let requests: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT id, from_user_id, message, "created-at"
           FROM friend_requests
           WHERE to_user_id = $1 AND status = 'pending'
           ORDER BY "created-at" DESC"#,
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AuthError::InternalServerError)?;

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
