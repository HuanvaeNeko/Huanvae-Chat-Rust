use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::AvatarUploadResponse;
use crate::storage::services::AvatarService;
use axum::{
    extract::{Multipart, State},
    Extension, Json,
};
use tracing::{error, info};

/// POST /api/profile/avatar - 上传头像
pub async fn upload_avatar(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<AvatarUploadResponse>>, AppError> {
    let user_id = &auth_ctx.user_id;

    // 从 multipart 中读取文件
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("");

        if field_name == "avatar" || field_name == "file" {
            filename = field.file_name().map(|s| s.to_string());

            match field.bytes().await {
                Ok(data) => {
                    file_data = Some(data.to_vec());
                    break;
                }
                Err(e) => {
                    error!("Failed to read file data: {}", e);
                    return Err(AppError::BadRequest("读取文件数据失败".to_string()));
                }
            }
        }
    }

    // 验证是否有文件
    let data = file_data.ok_or_else(|| {
        AppError::BadRequest("未上传文件，请使用字段名 'avatar' 或 'file'".to_string())
    })?;

    let fname = filename.unwrap_or_else(|| "avatar.jpg".to_string());
    info!("Uploading avatar for user: {}, filename: {}", user_id, fname);

    // 上传到 MinIO
    let avatar_url = AvatarService::upload_avatar(&state.s3_client, user_id, data, &fname)
        .await
        .map_err(|e| {
            error!("Failed to upload avatar: {}", e);
            AppError::Storage(e.to_string())
        })?;

    // 更新数据库
    state
        .profile_service
        .update_avatar_url(user_id, &avatar_url)
        .await?;

    let response = AvatarUploadResponse {
        avatar_url,
        message: "头像上传成功".to_string(),
    };

    Ok(Json(ApiResponse::success(response)))
}

