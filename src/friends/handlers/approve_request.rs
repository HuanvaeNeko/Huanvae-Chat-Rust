use axum::{
    extract::{Extension, State},
    Json,
};

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends::handlers::state::FriendsState;
use crate::friends::models::ApproveFriendRequest;
use crate::friends::services::approve_request;

pub async fn approve_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<ApproveFriendRequest>,
) -> Result<(), AppError> {
    approve_request(&state.service, &auth, body).await
}