use crate::auth::middleware::AuthContext;
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::UpdatePasswordRequest;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::json;
use tracing::error;
use validator::Validate;

/// PUT /api/profile/password - 修改密码
pub async fn update_password(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<UpdatePasswordRequest>,
) -> impl IntoResponse {
    let user_id = &auth_ctx.user_id;

    // 验证请求
    if let Err(e) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Validation error: {}", e) })),
        );
    }

    match state.profile_service.update_password(user_id, request).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Password updated successfully" })),
        ),
        Err(e) => {
            error!("Failed to update password: {}", e);
            let status = if e.to_string().contains("incorrect") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": e.to_string() })))
        }
    }
}

