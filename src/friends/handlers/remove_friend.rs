use axum::{extract::{State, Extension}, Json};
use crate::friends::models::RemoveFriendRequest;
use crate::friends::services::{FriendsState, remove_friend};
use crate::auth::middleware::AuthContext;

pub async fn remove_friend_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RemoveFriendRequest>,
) -> Result<(), crate::auth::errors::AuthError> {
    remove_friend(&state, &auth, body).await
}