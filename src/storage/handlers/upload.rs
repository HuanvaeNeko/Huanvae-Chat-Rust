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
use crate::friends::services::verify_friendship;
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::FileService;

/// Storage状态
#[derive(Clone)]
pub struct StorageState {
    pub db: PgPool,
    pub file_service: Arc<FileService>,
    pub s3_client: Arc<S3Client>,
}

impl StorageState {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>, api_base_url: String) -> Self {
        let file_service = Arc::new(FileService::new(db.clone(), s3_client.clone(), api_base_url));
        Self {
            db,
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

    // 好友文件上传：验证好友关系
    if request.storage_location == StorageLocation::FriendMessages {
        let friend_id = request.related_id.as_ref().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "好友ID不能为空" })),
            )
        })?;

        match verify_friendship(&state.db, &auth_ctx.user_id, friend_id).await {
            Ok(is_friend) => {
                if !is_friend {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": "不是好友关系，无法上传文件" })),
                    ));
                }
            }
            Err(e) => {
                error!("验证好友关系失败: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "验证好友关系失败" })),
                ));
            }
        }
    }

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

    // 3. 跳过哈希验证（采样哈希无法在服务端验证）
    // 采样哈希由客户端计算，服务端仅用于去重检查
    info!("文件上传成功，采样哈希: {}", file_record.file_hash);

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

    // 5. 上传到MinIO（但不再使用MinIO URL作为file_url）
    let storage_loc: StorageLocation = file_record.storage_location.parse()
        .map_err(|e: String| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ))?;
    let bucket = state.file_service.get_bucket_name(&storage_loc);
    let _minio_url = match state.s3_client
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

    // 6. 标记为完成并消费Token，创建UUID映射
    match state.file_service
        .complete_upload_with_token(
            &params.token, 
            &file_record.file_hash,  // 使用客户端提供的采样哈希
            &file_record.file_key,
            &file_record.owner_id,
            file_record.file_size,
            &file_record.content_type,
            &file_record.preview_support
        )
        .await
    {
        Ok(uuid_file_url) => {
            info!("文件上传成功并创建UUID映射: {}", file_record.file_key);
            let preview_support = file_record.preview_support();
            Ok(Json(FileCompleteResponse {
                file_url: uuid_file_url,  // 返回UUID访问URL
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

