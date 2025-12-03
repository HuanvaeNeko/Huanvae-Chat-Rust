use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::auth::middleware::AuthContext;
use crate::friends::services::verify_friendship;
use crate::storage::handlers::upload::StorageState;
use crate::storage::models::{PresignedUrlRequest, PresignedUrlResponse};

/// 从 conversation_uuid 解析出两个用户ID
/// conversation_uuid 格式: "conv-{user_id_1}-{user_id_2}"
fn parse_conversation_participants(conversation_uuid: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = conversation_uuid.splitn(3, '-').collect();
    if parts.len() == 3 && parts[0] == "conv" {
        Some((parts[1].to_string(), parts[2].to_string()))
    } else {
        None
    }
}

/// 从 physical_file_key 提取 conversation_uuid
/// physical_file_key 格式: "conv-{user1}-{user2}/images/xxx.jpg"
fn extract_conversation_uuid(physical_file_key: &str) -> Option<String> {
    let parts: Vec<&str> = physical_file_key.split('/').collect();
    if !parts.is_empty() && parts[0].starts_with("conv-") {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// POST /api/storage/friends-file/{uuid}/presigned-url
/// 生成好友文件的预签名URL（3小时有效）
pub async fn generate_friend_file_presigned_url(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(file_uuid): Path<String>,
    Json(request): Json<PresignedUrlRequest>,
) -> Result<Json<PresignedUrlResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 请求好友文件 {} 的预签名URL", auth_ctx.user_id, file_uuid);

    // 使用默认3小时（10800秒）
    let expires_in = request.expires_in.unwrap_or(10800);
    
    // 限制最大有效期为3小时
    if expires_in > 10800 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "普通文件预签名URL最长有效期为3小时" })),
        ));
    }

    // 1. 查询 file-uuid-mapping 获取物理文件信息
    let mapping = state.file_service
        .get_uuid_mapping(&file_uuid)
        .await
        .map_err(|e| {
            error!("查询文件映射失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "查询文件失败" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "文件不存在" })),
            )
        })?;

    // 2. 从 physical_file_key 提取 conversation_uuid
    let conversation_uuid = extract_conversation_uuid(&mapping.physical_file_key)
        .ok_or_else(|| {
            error!("无法从文件路径解析 conversation_uuid: {}", mapping.physical_file_key);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "文件路径格式错误，无法识别会话" })),
            )
        })?;

    // 3. 解析会话参与者
    let (user_a, user_b) = parse_conversation_participants(&conversation_uuid)
        .ok_or_else(|| {
            error!("无法解析会话参与者: {}", conversation_uuid);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "会话标识格式错误" })),
            )
        })?;

    // 4. 验证请求用户是否是会话参与者
    let user_id = auth_ctx.user_id.to_string();
    if user_id != user_a && user_id != user_b {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "无权访问此文件" })),
        ));
    }

    // 5. 确定好友ID
    let friend_id = if user_id == user_a { &user_b } else { &user_a };

    // 6. 验证当前好友关系
    match verify_friendship(&state.db, &user_id, friend_id).await {
        Ok(is_friend) => {
            if !is_friend {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "好友关系已解除，无法访问文件" })),
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

    // 7. 生成预签名URL
    let presigned_url = state.s3_client
        .generate_presigned_download_url("friends-file", &mapping.physical_file_key, expires_in)
        .await
        .map_err(|e| {
            error!("生成预签名URL失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "生成预签名URL失败" })),
            )
        })?;

    // 8. 计算过期时间
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    Ok(Json(PresignedUrlResponse {
        presigned_url,
        expires_at: expires_at.to_rfc3339(),
        file_uuid,
        file_size: mapping.file_size,
        content_type: mapping.content_type,
        warning: None,
    }))
}

/// POST /api/storage/friends-file/{uuid}/presigned-url/extended
/// 生成好友文件的扩展预签名URL（超大文件，自定义有效期）
pub async fn generate_friend_file_extended_presigned_url(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(file_uuid): Path<String>,
    Json(request): Json<PresignedUrlRequest>,
) -> Result<Json<PresignedUrlResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 请求好友超大文件 {} 的扩展预签名URL", auth_ctx.user_id, file_uuid);

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

    // 1. 查询 file-uuid-mapping 获取物理文件信息
    let mapping = state.file_service
        .get_uuid_mapping(&file_uuid)
        .await
        .map_err(|e| {
            error!("查询文件映射失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "查询文件失败" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "文件不存在" })),
            )
        })?;

    // 2. 从 physical_file_key 提取 conversation_uuid
    let conversation_uuid = extract_conversation_uuid(&mapping.physical_file_key)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "文件路径格式错误，无法识别会话" })),
            )
        })?;

    // 3. 解析会话参与者
    let (user_a, user_b) = parse_conversation_participants(&conversation_uuid)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "会话标识格式错误" })),
            )
        })?;

    // 4. 验证请求用户是否是会话参与者
    let user_id = auth_ctx.user_id.to_string();
    if user_id != user_a && user_id != user_b {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "无权访问此文件" })),
        ));
    }

    // 5. 确定好友ID并验证好友关系
    let friend_id = if user_id == user_a { &user_b } else { &user_a };

    match verify_friendship(&state.db, &user_id, friend_id).await {
        Ok(is_friend) => {
            if !is_friend {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "好友关系已解除，无法访问文件" })),
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

    // 6. 生成预签名URL
    let presigned_url = state.s3_client
        .generate_presigned_download_url("friends-file", &mapping.physical_file_key, expires_in)
        .await
        .map_err(|e| {
            error!("生成预签名URL失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "生成预签名URL失败" })),
            )
        })?;

    // 7. 计算过期时间和警告信息
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);
    let warning = if expires_in > 86400 {
        Some(format!("此链接将在{}小时后过期", expires_in / 3600))
    } else {
        None
    };

    Ok(Json(PresignedUrlResponse {
        presigned_url,
        expires_at: expires_at.to_rfc3339(),
        file_uuid,
        file_size: mapping.file_size,
        content_type: mapping.content_type,
        warning,
    }))
}

