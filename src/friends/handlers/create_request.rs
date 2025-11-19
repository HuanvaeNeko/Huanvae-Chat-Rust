use axum::{extract::{State, Extension}, Json};
use crate::friends::models::{SubmitFriendRequest, SubmitFriendResponse};
use crate::friends::services::{FriendsState, submit_request};
use crate::auth::middleware::AuthContext;

pub async fn create_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<SubmitFriendRequest>,
) -> Result<Json<SubmitFriendResponse>, crate::auth::errors::AuthError> {
    let resp = submit_request(&state, &auth, body).await?;
    Ok(Json(resp))
}