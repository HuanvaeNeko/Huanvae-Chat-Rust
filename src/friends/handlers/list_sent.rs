use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, SentRequestDto};
use crate::auth::middleware::extract_auth_context;
use crate::common::AppError;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_sent_requests_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<SentRequestDto>>, AppError> {
    let auth = extract_auth_context(&request)?;
    
    // 查询我发出的待处理好友请求
    let requests: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT "id", "to-user-id", "message", "created-at"
           FROM "friend-requests"
           WHERE "from-user-id" = $1 AND "status" = 'pending'
           ORDER BY "created-at" DESC"#,
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询已发送好友请求失败: {}", e)))?;

    let items = requests
        .into_iter()
        .map(|(id, to_user_id, message, created_at)| SentRequestDto {
            request_id: id.to_string(),
            sent_to_user_id: to_user_id,
            sent_message: if message.is_empty() { None } else { Some(message) },
            sent_time: created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ListResponse { items }))
}
