use axum::{extract::{State, Request}, Json};
use crate::friends::models::{ListResponse, FriendDto};
use crate::auth::middleware::extract_auth_context;
use crate::common::AppError;
use sqlx::PgPool;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct ListState { pub db: PgPool }

pub async fn list_owned_friends_handler(
    State(state): State<ListState>,
    request: Request,
) -> Result<Json<ListResponse<FriendDto>>, AppError> {
    let auth = extract_auth_context(&request)?;
    
    // 查询好友关系表，关联用户表获取昵称和头像
    let friends: Vec<(String, Option<String>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT f."friend-id", u."user-nickname", u."user-avatar-url", f."add-time"
           FROM "friendships" f
           LEFT JOIN "users" u ON u."user-id" = f."friend-id"
           WHERE f."user-id" = $1 AND f."status" = 'active'
           ORDER BY f."add-time" DESC"#,
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    let items = friends
        .into_iter()
        .map(|(friend_id, nickname, avatar_url, add_time)| FriendDto {
            friend_id,
            friend_nickname: nickname,
            friend_avatar_url: avatar_url,
            add_time: add_time.to_rfc3339(),
            approve_reason: None,
        })
        .collect();

    Ok(Json(ListResponse { items }))
}
