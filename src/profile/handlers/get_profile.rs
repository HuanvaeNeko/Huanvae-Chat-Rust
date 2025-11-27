use crate::auth::middleware::AuthContext;
use crate::profile::handlers::routes::ProfileAppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json;
use tracing::error;

/// GET /api/profile - 获取当前用户信息
pub async fn get_profile(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let user_id = &auth_ctx.user_id;

    match state.profile_service.get_profile(user_id).await {
        Ok(profile) => (StatusCode::OK, Json(json!({ "data": profile }))),
        Err(e) => {
            error!("Failed to get profile: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch profile" })),
            )
        }
    }
}

