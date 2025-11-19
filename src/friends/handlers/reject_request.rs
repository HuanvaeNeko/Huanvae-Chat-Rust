use axum::{extract::{State, Extension}, Json};
use crate::friends::models::RejectFriendRequest;
use crate::friends::services::{FriendsState, reject_request};
use crate::auth::middleware::AuthContext;

pub async fn reject_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RejectFriendRequest>,
) -> Result<(), crate::auth::errors::AuthError> {
    reject_request(&state, &auth, body).await
}