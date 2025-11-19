use axum::{extract::{State, Extension}, Json};
use crate::friends::models::ApproveFriendRequest;
use crate::friends::services::{FriendsState, approve_request};
use crate::auth::middleware::AuthContext;

pub async fn approve_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<ApproveFriendRequest>,
) -> Result<(), crate::auth::errors::AuthError> {
    approve_request(&state, &auth, body).await
}