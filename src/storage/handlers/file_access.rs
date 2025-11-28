use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Extension,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

use crate::auth::middleware::AuthContext;
use crate::storage::client::S3Client;
use crate::storage::services::UuidMappingService;

/// Storage状态（用于文件访问）
#[derive(Clone)]
pub struct FileAccessState {
    pub uuid_mapping_service: Arc<UuidMappingService>,
    pub s3_client: Arc<S3Client>,
}

/// GET /api/storage/file/{uuid} - 通过UUID访问文件
pub async fn get_file_by_uuid(
    State(state): State<FileAccessState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(uuid): Path<String>,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    let user_id = &auth_ctx.user_id.to_string();
    
    info!("用户 {} 请求访问文件 UUID: {}", user_id, uuid);
    
    // 1. 检查权限
    match state.uuid_mapping_service.check_permission(&uuid, user_id).await {
        Ok(has_permission) => {
            if !has_permission {
                error!("用户 {} 无权访问文件 {}", user_id, uuid);
                return Err((
                    StatusCode::FORBIDDEN,
                    axum::Json(json!({ "error": "无权访问此文件" })),
                ));
            }
        }
        Err(e) => {
            error!("权限检查失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "权限检查失败" })),
            ));
        }
    }
    
    // 2. 获取映射信息
    let mapping = match state.uuid_mapping_service.get_by_uuid(&uuid).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            error!("文件UUID不存在: {}", uuid);
            return Err((
                StatusCode::NOT_FOUND,
                axum::Json(json!({ "error": "文件不存在" })),
            ));
        }
        Err(e) => {
            error!("获取映射信息失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "获取文件信息失败" })),
            ));
        }
    };
    
    // 3. 确定bucket
    let bucket = if mapping.physical_file_key.contains('/') {
        "user-file"
    } else {
        "avatars"
    };
    
    // 4. 从MinIO读取文件
    match state.s3_client.get_file(bucket, &mapping.physical_file_key).await {
        Ok(file_data) => {
            info!("文件读取成功: {} ({}字节)", mapping.physical_file_key, file_data.len());
            
            // 5. 返回文件流
            Ok((
                [
                    (header::CONTENT_TYPE, mapping.content_type.as_str()),
                    (header::CONTENT_LENGTH, &file_data.len().to_string()),
                    (header::CACHE_CONTROL, "public, max-age=31536000"),
                ],
                file_data,
            ).into_response())
        }
        Err(e) => {
            error!("从MinIO读取文件失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "读取文件失败" })),
            ))
        }
    }
}

