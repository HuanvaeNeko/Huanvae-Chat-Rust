use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::UpdatePasswordRequest;
use axum::{extract::State, Extension, Json};
use tracing::{error, info};
use validator::Validate;

/// PUT /api/profile/password - 修改密码
pub async fn update_password(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<UpdatePasswordRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let user_id = &auth_ctx.user_id;

    // 验证请求
    if let Err(e) = request.validate() {
        return Err(AppError::ValidationError(e.to_string()));
    }

    // 更新密码
    state
        .profile_service
        .update_password(user_id, request)
        .await?;

    // 密码修改成功后，拉黑用户所有 Access Token
    match state
        .blacklist_service
        .blacklist_all_user_access_tokens(user_id, "密码已修改")
        .await
    {
        Ok(count) => {
            info!(
                "✅ 用户 {} 密码修改成功，已拉黑 {} 个 Access Token",
                user_id, count
            );
        }
        Err(e) => {
            // 拉黑失败不影响密码修改结果，但记录错误
            error!("⚠️ 拉黑 Access Token 失败: {}", e);
        }
    }

    Ok(ApiResponse::ok("密码修改成功"))
}

