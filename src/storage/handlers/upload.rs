use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};

use crate::auth::middleware::AuthContext;
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::{compute_sha256, FileService};

/// Storage状态
#[derive(Clone)]
pub struct StorageState {
    pub file_service: Arc<FileService>,
    pub s3_client: Arc<S3Client>,
}

impl StorageState {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>, api_base_url: String) -> Self {
        let file_service = Arc::new(FileService::new(db, s3_client.clone(), api_base_url));
        Self {
            file_service,
            s3_client,
        }
    }
}

/// POST /api/storage/upload/request - 请求上传
pub async fn request_upload(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<FileUploadRequest>,
) -> Result<Json<FileUploadResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 请求上传文件: {}", auth_ctx.user_id, request.filename);

    match state.file_service
        .request_upload(&auth_ctx.user_id.to_string(), request)
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("请求上传失败: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ))
        }
    }
}

/// POST /api/storage/upload/direct?token={token}
/// 直接上传文件（通过一次性Token验证）
pub async fn direct_upload(
    State(state): State<StorageState>,
    Query(params): Query<DirectUploadQuery>,
    mut multipart: Multipart,
) -> Result<Json<FileCompleteResponse>, (StatusCode, Json<Value>)> {
    // 1. 验证Token并获取文件信息
    let file_record = match state.file_service
        .verify_and_get_upload_token(&params.token)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            error!("Token验证失败: {}", e);
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": format!("Token无效: {}", e) })),
            ));
        }
    };

    info!("开始上传文件: {}", file_record.file_key);

    // 2. 读取文件数据
    let mut file_data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            match field.bytes().await {
                Ok(data) => {
                    file_data = Some(data.to_vec());
                    break;
                }
                Err(e) => {
                    error!("读取文件失败: {}", e);
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("读取文件失败: {}", e) })),
                    ));
                }
            }
        }
    }

    let data = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "未找到文件数据，请使用字段名'file'" })),
        )
    })?;

    // 3. 验证文件哈希
    let actual_hash = compute_sha256(&data);
    if actual_hash != file_record.file_hash {
        error!("文件哈希不匹配: 期望 {}, 实际 {}", file_record.file_hash, actual_hash);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ 
                "error": "文件哈希不匹配，文件可能已损坏",
                "expected": file_record.file_hash,
                "actual": actual_hash
            })),
        ));
    }

    // 4. 验证文件大小
    if data.len() as i64 != file_record.file_size {
        error!("文件大小不匹配: 期望 {}, 实际 {}", file_record.file_size, data.len());
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ 
                "error": "文件大小不匹配",
                "expected": file_record.file_size,
                "actual": data.len()
            })),
        ));
    }

    // 5. 上传到MinIO
    let storage_loc: StorageLocation = file_record.storage_location.parse()
        .map_err(|e: String| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ))?;
    let bucket = state.file_service.get_bucket_name(&storage_loc);
    let file_url = match state.s3_client
        .upload_file(bucket, &file_record.file_key, data, &file_record.content_type)
        .await
    {
        Ok(url) => url,
        Err(e) => {
            error!("上传到MinIO失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "上传失败" })),
            ));
        }
    };

    // 6. 标记为完成并消费Token
    match state.file_service
        .complete_upload_with_token(&params.token, &actual_hash)
        .await
    {
        Ok(_) => {
            info!("文件上传成功: {}", file_record.file_key);
            let preview_support = file_record.preview_support();
            Ok(Json(FileCompleteResponse {
                file_url,
                file_key: file_record.file_key.clone(),
                file_size: file_record.file_size as u64,
                content_type: file_record.content_type.clone(),
                preview_support,
            }))
        }
        Err(e) => {
            error!("完成上传失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "完成上传失败" })),
            ))
        }
    }
}

/// GET /api/storage/multipart/part-url
/// 获取分片上传的预签名URL
pub async fn get_multipart_part_url(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<MultipartPartRequest>,
) -> Result<Json<MultipartPartResponse>, (StatusCode, Json<Value>)> {
    match state.file_service
        .generate_multipart_part_url(
            &params.file_key,
            &params.upload_id,
            params.part_number,
            &auth_ctx.user_id.to_string(),
        )
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("生成分片URL失败: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DirectUploadQuery {
    pub token: String,
}

