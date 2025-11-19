use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, PendingRequestDto};
use crate::friends::services::parse_records;
use crate::auth::{errors::AuthError, middleware::extract_auth_context};
use sqlx::PgPool;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_pending_requests_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<PendingRequestDto>>, AuthError> {
    let auth = extract_auth_context(&request)?;
    let (pending_text,): (String,) = sqlx::query_as(
        r#"SELECT "user-pending-friend-requests" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let items = parse_records(&pending_text)
        .into_iter()
        .filter(|r| r.get("status").map(|s| s == "open").unwrap_or(true))
        .map(|r| PendingRequestDto {
            request_id: r.get("request-id").cloned().unwrap_or_default(),
            request_user_id: r.get("request-user-id").cloned().unwrap_or_default(),
            request_message: r.get("request-message").cloned(),
            request_time: r.get("request-time").cloned().unwrap_or_default(),
        })
        .collect();

    Ok(Json(ListResponse { items }))
}