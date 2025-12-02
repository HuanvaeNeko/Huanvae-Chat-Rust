use axum::{
    extract::{Extension, State},
    Json,
};

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends::handlers::state::FriendsState;
use crate::friends::models::RemoveFriendRequest;
use crate::friends::services::remove_friend;

pub async fn remove_friend_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RemoveFriendRequest>,
) -> Result<(), AppError> {
    remove_friend(&state.service, &auth, body).await
}