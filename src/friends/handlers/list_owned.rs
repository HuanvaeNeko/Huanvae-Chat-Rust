use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, FriendDto};
use crate::auth::{errors::AuthError, middleware::extract_auth_context};
use sqlx::PgPool;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_owned_friends_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<FriendDto>>, AuthError> {
    let auth = extract_auth_context(&request)?;
    
    // 查询好友关系表，关联用户表获取昵称
    let friends: Vec<(String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT f.friend_id, u."user-nickname", f.add_time
           FROM friendships f
           LEFT JOIN users u ON u."user-id" = f.friend_id
           WHERE f.user_id = $1 AND f.status = 'active'
           ORDER BY f.add_time DESC"#,
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AuthError::InternalServerError)?;

    let items = friends
        .into_iter()
        .map(|(friend_id, nickname, add_time)| FriendDto {
            friend_id,
            friend_nickname: nickname,
            add_time: add_time.to_rfc3339(),
            approve_reason: None,
        })
        .collect();

    Ok(Json(ListResponse { items }))
}
