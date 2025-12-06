//! 房间 CRUD API

use axum::{extract::State, Extension, Json};

use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::webrtc_room::models::{CreateRoomRequest, CreateRoomResponse};

use super::WebRTCState;

/// 创建房间
///
/// POST /api/webrtc/rooms
pub async fn create_room(
    State(state): State<WebRTCState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<ApiResponse<CreateRoomResponse>>, AppError> {
    let response = state.room_service.create_room(&auth.user_id, request);

    tracing::info!(
        room_id = %response.room_id,
        creator_id = %auth.user_id,
        "房间已创建"
    );

    Ok(Json(ApiResponse::success(response)))
}

