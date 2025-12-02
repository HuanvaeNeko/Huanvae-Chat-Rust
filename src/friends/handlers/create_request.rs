use axum::{
    extract::{Extension, State},
    Json,
};

use crate::auth::middleware::AuthContext;
use crate::common::AppError;
use crate::friends::handlers::state::FriendsState;
use crate::friends::models::{SubmitFriendRequest, SubmitFriendResponse};
use crate::friends::services::submit_request;

pub async fn create_friend_request_handler(
    State(state): State<FriendsState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<SubmitFriendRequest>,
) -> Result<Json<SubmitFriendResponse>, AppError> {
    let resp = submit_request(&state.service, &auth, body).await?;
    Ok(Json(resp))
}