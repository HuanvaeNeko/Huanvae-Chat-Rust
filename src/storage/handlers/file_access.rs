use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::auth::middleware::AuthContext;
use crate::storage::handlers::upload::StorageState;
use crate::storage::models::{PresignedUrlRequest, PresignedUrlResponse};

/// POST /api/storage/file/{uuid}/presigned_url
/// 生成普通文件的预签名URL（3小时有效）
pub async fn generate_presigned_url(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(uuid): Path<String>,
    Json(request): Json<PresignedUrlRequest>,
) -> Result<Json<PresignedUrlResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 请求文件 {} 的预签名URL", auth_ctx.user_id, uuid);

    // 使用默认3小时（10800秒）
    let expires_in = request.expires_in.unwrap_or(10800);
    
    // 限制最大有效期为3小时
    if expires_in > 10800 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "普通文件预签名URL最长有效期为3小时" })),
        ));
    }

    match state
        .file_service
        .generate_presigned_url(&auth_ctx.user_id.to_string(), &uuid, expires_in)
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("生成预签名URL失败: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ))
        }
    }
}

/// POST /api/storage/file/{uuid}/presigned_url/extended
/// 生成超大文件的预签名URL（自定义有效时间）
pub async fn generate_extended_presigned_url(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(uuid): Path<String>,
    Json(request): Json<PresignedUrlRequest>,
) -> Result<Json<PresignedUrlResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 请求超大文件 {} 的扩展预签名URL", auth_ctx.user_id, uuid);

    let expires_in = request
        .estimated_download_time
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "超大文件必须指定 estimated_download_time" })),
            )
        })?;

    // 限制：最少3小时，最多7天
    if expires_in < 10800 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "有效期最少为3小时（10800秒）" })),
        ));
    }
    
    if expires_in > 604800 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "有效期最多为7天（604800秒）" })),
        ));
    }

    match state
        .file_service
        .generate_presigned_url(&auth_ctx.user_id.to_string(), &uuid, expires_in)
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("生成扩展预签名URL失败: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ))
        }
    }
}
