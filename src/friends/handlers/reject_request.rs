use axum::{
    extract::{Extension, State},
    Json,
};

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends::handlers::state::FriendsState;
use crate::friends::models::RejectFriendRequest;
use crate::friends::services::reject_request;

pub async fn reject_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<RejectFriendRequest>,
) -> Result<(), AppError> {
    reject_request(&state.service, &auth, body).await
}