use crate::auth::middleware::AuthContext;
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::UpdateProfileRequest;
use crate::profile::services::ProfileService;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json;
use tracing::error;
use validator::Validate;

/// PUT /api/profile - 更新个人信息（邮箱、签名）
pub async fn update_profile(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let user_id = &auth_ctx.user_id;

    // 验证请求
    if let Err(e) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Validation error: {}", e) })),
        );
    }

    match ProfileService::update_profile(&state.pool, user_id, request).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Profile updated successfully" })),
        ),
        Err(e) => {
            error!("Failed to update profile: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        }
    }
}

