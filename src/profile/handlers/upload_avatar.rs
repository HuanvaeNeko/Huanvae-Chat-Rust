use crate::auth::middleware::AuthContext;
use crate::profile::handlers::routes::ProfileAppState;
use crate::profile::models::AvatarUploadResponse;
use crate::storage::services::AvatarService;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use tracing::{error, info};

/// POST /api/profile/avatar - 上传头像
pub async fn upload_avatar(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
    mut multipart: Multipart,
) -> impl IntoResponse {
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
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": "Failed to read file data" })),
                    );
                }
            }
        }
    }

    // 验证是否有文件
    let data = match file_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "No file uploaded. Use field name 'avatar' or 'file'" })),
            );
        }
    };

    let fname = filename.unwrap_or_else(|| "avatar.jpg".to_string());
    info!("Uploading avatar for user: {}, filename: {}", user_id, fname);

    // 上传到 MinIO
    match AvatarService::upload_avatar(&state.s3_client, user_id, data, &fname).await {
        Ok(avatar_url) => {
            // 更新数据库
            match state.profile_service.update_avatar_url(user_id, &avatar_url).await {
                Ok(_) => {
                    let response = AvatarUploadResponse {
                        avatar_url: avatar_url.clone(),
                        message: "Avatar uploaded successfully".to_string(),
                    };
                    (StatusCode::OK, Json(json!(response)))
                }
                Err(e) => {
                    error!("Failed to update avatar URL in database: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to save avatar URL" })),
                    )
                }
            }
        }
        Err(e) => {
            error!("Failed to upload avatar: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
        }
    }
}

