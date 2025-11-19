use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, FriendDto};
use crate::friends::services::parse_records;
use crate::auth::{errors::AuthError, middleware::extract_auth_context};
use sqlx::PgPool;

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_owned_friends_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<FriendDto>>, AuthError> {
    let auth = extract_auth_context(&request)?;
    let (friends_text,): (String,) = sqlx::query_as(
        r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let items = parse_records(&friends_text)
        .into_iter()
        .filter(|r| r.get("status").map(|s| s == "active").unwrap_or(false))
        .map(|r| FriendDto {
            friend_id: r.get("friend-id").cloned().unwrap_or_default(),
            friend_nickname: r.get("friend-nickname").cloned(),
            add_time: r.get("add-time").cloned().unwrap_or_default(),
            approve_reason: r.get("approve-reason").cloned(),
        })
        .collect();

    Ok(Json(ListResponse { items }))
}