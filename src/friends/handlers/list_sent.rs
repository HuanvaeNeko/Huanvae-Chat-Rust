use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, SentRequestDto};
use crate::friends::services::parse_records;
use crate::auth::{errors::AuthError, middleware::extract_auth_context};
use sqlx::PgPool;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_sent_requests_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<SentRequestDto>>, AuthError> {
    let auth = extract_auth_context(&request)?;
    let (sent_text,): (String,) = sqlx::query_as(
        r#"SELECT "user-sent-friend-requests" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let items = parse_records(&sent_text)
        .into_iter()
        .filter(|r| r.get("status").map(|s| s == "open").unwrap_or(true))
        .map(|r| SentRequestDto {
            request_id: r.get("request-id").cloned().unwrap_or_default(),
            sent_to_user_id: r.get("sent-to-user-id").cloned().unwrap_or_default(),
            sent_message: r.get("sent-message").cloned(),
            sent_time: r.get("sent-time").cloned().unwrap_or_default(),
        })
        .collect();

    Ok(Json(ListResponse { items }))
}