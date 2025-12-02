use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::ProfileResponse;
use axum::{extract::State, Extension, Json};

/// GET /api/profile - 获取当前用户信息
pub async fn get_profile(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<ApiResponse<ProfileResponse>>, AppError> {
    let user_id = &auth_ctx.user_id;

    let profile = state.profile_service.get_profile(user_id).await?;
    Ok(Json(ApiResponse::success(profile)))
}

