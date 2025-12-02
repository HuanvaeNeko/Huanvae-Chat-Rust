use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::UpdateProfileRequest;
use axum::{extract::State, Extension, Json};
use validator::Validate;

/// PUT /api/profile - 更新个人信息（邮箱、签名）
pub async fn update_profile(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<UpdateProfileRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let user_id = &auth_ctx.user_id;

    // 验证请求
    if let Err(e) = request.validate() {
        return Err(AppError::ValidationError(e.to_string()));
    }

    state.profile_service.update_profile(user_id, request).await?;
    Ok(ApiResponse::ok("个人信息更新成功"))
}

