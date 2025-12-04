use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::friends::services::verify_friendship;
use crate::friends_messages::services::MessageService;
use crate::group_messages::services::GroupMessageService;
use crate::groups::services::MemberService as GroupMemberService;
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::FileService;
use crate::websocket::NotificationService;

/// Storage状态
#[derive(Clone)]
pub struct StorageState {
    pub db: PgPool,
    pub file_service: Arc<FileService>,
    pub s3_client: Arc<S3Client>,
    pub message_service: MessageService,
    pub group_message_service: GroupMessageService,
    pub group_member_service: GroupMemberService,
    pub notification_service: Option<NotificationService>,
}

impl StorageState {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>, api_base_url: String) -> Self {
        let file_service = Arc::new(FileService::new(db.clone(), s3_client.clone(), api_base_url));
        let message_service = MessageService::new(db.clone());
        let group_message_service = GroupMessageService::new(db.clone());
        let group_member_service = GroupMemberService::new(db.clone());
        Self {
            db,
            file_service,
            s3_client,
            message_service,
            group_message_service,
            group_member_service,
            notification_service: None,
        }
    }
    
    /// 设置 NotificationService（用于文件上传后发送实时通知）
    pub fn with_notification(mut self, notification_service: NotificationService) -> Self {
        self.notification_service = Some(notification_service);
        self
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
    let is_friend_file = request.storage_location == StorageLocation::FriendMessages;
    if is_friend_file {
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

    // 群文件上传：验证群成员身份
    let is_group_file = request.storage_location == StorageLocation::GroupFiles;
    if is_group_file {
        let group_id_str = request.related_id.as_ref().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "群ID不能为空" })),
            )
        })?;

        let group_uuid = Uuid::parse_str(group_id_str).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "无效的群ID格式" })),
            )
        })?;

        match state.group_member_service.verify_active_member(&group_uuid, &auth_ctx.user_id).await {
            Ok(is_member) => {
                if !is_member {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": "您不是该群成员，无法上传文件" })),
                    ));
                }
            }
            Err(e) => {
                error!("验证群成员身份失败: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "验证群成员身份失败" })),
                ));
            }
        }
    }

    // 保存请求信息用于秒传后发送消息
    let filename = request.filename.clone();
    let content_type = request.content_type.clone();
    let file_size = request.file_size;
    let related_id = request.related_id.clone();

    match state.file_service
        .request_upload(&auth_ctx.user_id.to_string(), request)
        .await
    {
        Ok(mut response) => {
            // 好友文件秒传：自动发送消息
            if response.instant_upload && is_friend_file {
                if let Some(ref friend_id) = related_id {
                    // 提取 file_uuid
                    let file_uuid = response.existing_file_url.as_ref()
                        .and_then(|url| url.rsplit('/').next())
                        .unwrap_or("")
                        .to_string();
                    
                    // 确定消息类型和内容
                    let (message_type, message_content) = determine_message_type_and_content_simple(
                        &content_type,
                        &filename,
                    );
                    
                    info!("好友文件秒传完成，自动发送 {} 消息给 {}", message_type, friend_id);
                    
                    // 发送消息
                    match state.message_service.send_message(
                        &auth_ctx.user_id,
                        friend_id,
                        &message_content,
                        &message_type,
                        Some(file_uuid),
                        response.existing_file_url.clone(),
                        Some(file_size as i64),
                    ).await {
                        Ok((msg_uuid, send_time)) => {
                            info!("秒传文件消息已自动发送: {}", msg_uuid);
                            response.message_uuid = Some(msg_uuid.clone());
                            response.message_send_time = Some(send_time.clone());
                            
                            // 发送 WebSocket 实时通知
                            if let Some(ref notification_service) = state.notification_service {
                                let sender_nickname = get_user_nickname(&state.db, &auth_ctx.user_id).await;
                                let send_time_dt = chrono::DateTime::parse_from_rfc3339(&send_time)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                
                                if let Err(e) = notification_service.notify_friend_message(
                                    &auth_ctx.user_id,
                                    &sender_nickname,
                                    friend_id,
                                    &msg_uuid,
                                    &message_content,
                                    &message_type,
                                    send_time_dt,
                                ).await {
                                    warn!("秒传文件 WebSocket 通知失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("秒传文件消息发送失败: {}", e);
                        }
                    }
                }
            }

            // 群文件秒传：自动发送群消息
            if response.instant_upload && is_group_file {
                if let Some(ref group_id_str) = related_id {
                    if let Ok(group_uuid) = Uuid::parse_str(group_id_str) {
                        // 提取 file_uuid
                        let file_uuid = response.existing_file_url.as_ref()
                            .and_then(|url| url.rsplit('/').next())
                            .unwrap_or("")
                            .to_string();
                        
                        // 确定消息类型和内容
                        let (message_type, message_content) = determine_message_type_and_content_simple(
                            &content_type,
                            &filename,
                        );
                        
                        info!("群文件秒传完成，自动发送 {} 消息到群 {}", message_type, group_id_str);
                        
                        // 发送群消息
                        match state.group_message_service.send_message(
                            &group_uuid,
                            &auth_ctx.user_id,
                            &message_content,
                            &message_type,
                            Some(&file_uuid),
                            response.existing_file_url.as_deref(),
                            Some(file_size as i64),
                            None,
                        ).await {
                            Ok(resp) => {
                                info!("群文件秒传消息已自动发送: {}", resp.message_uuid);
                                response.message_uuid = Some(resp.message_uuid.clone());
                                response.message_send_time = Some(resp.send_time.clone());
                                
                                // 发送 WebSocket 实时通知
                                if let Some(ref notification_service) = state.notification_service {
                                    let sender_nickname = get_user_nickname(&state.db, &auth_ctx.user_id).await;
                                    let send_time_dt = chrono::DateTime::parse_from_rfc3339(&resp.send_time)
                                        .map(|dt| dt.with_timezone(&Utc))
                                        .unwrap_or_else(|_| Utc::now());
                                    
                                    // 解析 message_uuid 为 Uuid
                                    if let Ok(msg_uuid) = Uuid::parse_str(&resp.message_uuid) {
                                        if let Err(e) = notification_service.notify_group_message(
                                            &group_uuid,
                                            "",  // group_name（群名称，通知服务不使用）
                                            &auth_ctx.user_id,
                                            &sender_nickname,
                                            &msg_uuid,
                                            &message_content,
                                            &message_type,
                                            send_time_dt,
                                        ).await {
                                            warn!("群文件秒传 WebSocket 通知失败: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("群文件秒传消息发送失败: {}", e);
                            }
                        }
                    }
                }
            }
            
            Ok(Json(response))
        }
        Err(e) => {
            error!("请求上传失败: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ))
        }
    }
}

/// 根据文件类型确定消息类型和内容（简化版，用于秒传）
fn determine_message_type_and_content_simple(content_type: &str, filename: &str) -> (String, String) {
    if content_type.starts_with("image/") {
        ("image".to_string(), format!("[图片] {}", filename))
    } else if content_type.starts_with("video/") {
        ("video".to_string(), format!("[视频] {}", filename))
    } else {
        ("file".to_string(), format!("[文件] {}", filename))
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
            
            // 提取 file_uuid 从 uuid_file_url
            let file_uuid = uuid_file_url
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();
            
            // 7. 好友文件：自动发送文件消息
            let mut message_uuid: Option<String> = None;
            let mut message_send_time: Option<String> = None;
            
            if storage_loc == StorageLocation::FriendMessages {
                if let Some(friend_id) = &file_record.related_id {
                    // 根据文件类型确定消息类型和内容
                    let (message_type, message_content) = determine_message_type_and_content(
                        &file_record.content_type,
                        &file_record.file_key,
                    );
                    
                    info!("好友文件上传完成，自动发送 {} 消息给 {}", message_type, friend_id);
                    
                    match state.message_service.send_message(
                        &file_record.owner_id,
                        friend_id,
                        &message_content,
                        &message_type,
                        Some(file_uuid.clone()),
                        Some(uuid_file_url.clone()),
                        Some(file_record.file_size),
                    ).await {
                        Ok((msg_uuid, send_time)) => {
                            info!("自动发送文件消息成功: {}", msg_uuid);
                            message_uuid = Some(msg_uuid.clone());
                            message_send_time = Some(send_time.clone());
                            
                            // 发送 WebSocket 实时通知
                            if let Some(ref notification_service) = state.notification_service {
                                let sender_nickname = get_user_nickname(&state.db, &file_record.owner_id).await;
                                let send_time_dt = chrono::DateTime::parse_from_rfc3339(&send_time)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                
                                if let Err(e) = notification_service.notify_friend_message(
                                    &file_record.owner_id,
                                    &sender_nickname,
                                    friend_id,
                                    &msg_uuid,
                                    &message_content,
                                    &message_type,
                                    send_time_dt,
                                ).await {
                                    warn!("好友文件上传 WebSocket 通知失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("自动发送文件消息失败: {}", e);
                            // 消息发送失败不影响文件上传结果
                        }
                    }
                }
            }

            // 8. 群文件：自动发送群消息
            if storage_loc == StorageLocation::GroupFiles {
                if let Some(group_id_str) = &file_record.related_id {
                    if let Ok(group_uuid) = Uuid::parse_str(group_id_str) {
                        // 根据文件类型确定消息类型和内容
                        let (message_type, message_content) = determine_message_type_and_content(
                            &file_record.content_type,
                            &file_record.file_key,
                        );
                        
                        info!("群文件上传完成，自动发送 {} 消息到群 {}", message_type, group_id_str);
                        
                        match state.group_message_service.send_message(
                            &group_uuid,
                            &file_record.owner_id,
                            &message_content,
                            &message_type,
                            Some(&file_uuid),
                            Some(&uuid_file_url),
                            Some(file_record.file_size),
                            None,
                        ).await {
                            Ok(resp) => {
                                info!("群文件消息自动发送成功: {}", resp.message_uuid);
                                message_uuid = Some(resp.message_uuid.clone());
                                message_send_time = Some(resp.send_time.clone());
                                
                                // 发送 WebSocket 实时通知
                                if let Some(ref notification_service) = state.notification_service {
                                    let sender_nickname = get_user_nickname(&state.db, &file_record.owner_id).await;
                                    let send_time_dt = chrono::DateTime::parse_from_rfc3339(&resp.send_time)
                                        .map(|dt| dt.with_timezone(&Utc))
                                        .unwrap_or_else(|_| Utc::now());
                                    
                                    // 解析 message_uuid 为 Uuid
                                    if let Ok(msg_uuid) = Uuid::parse_str(&resp.message_uuid) {
                                        if let Err(e) = notification_service.notify_group_message(
                                            &group_uuid,
                                            "",  // group_name（群名称，通知服务不使用）
                                            &file_record.owner_id,
                                            &sender_nickname,
                                            &msg_uuid,
                                            &message_content,
                                            &message_type,
                                            send_time_dt,
                                        ).await {
                                            warn!("群文件上传 WebSocket 通知失败: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("群文件消息自动发送失败: {}", e);
                                // 消息发送失败不影响文件上传结果
                            }
                        }
                    }
                }
            }
            
            Ok(Json(FileCompleteResponse {
                file_url: uuid_file_url,  // 返回UUID访问URL
                file_key: file_record.file_key.clone(),
                file_size: file_record.file_size as u64,
                content_type: file_record.content_type.clone(),
                preview_support,
                message_uuid,
                message_send_time,
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

/// 根据文件类型确定消息类型和内容
fn determine_message_type_and_content(content_type: &str, file_key: &str) -> (String, String) {
    // 从 file_key 提取文件名
    let filename = file_key
        .rsplit('/')
        .next()
        .and_then(|s| s.splitn(3, '_').nth(2))
        .unwrap_or("文件");
    
    if content_type.starts_with("image/") {
        ("image".to_string(), format!("[图片] {}", filename))
    } else if content_type.starts_with("video/") {
        ("video".to_string(), format!("[视频] {}", filename))
    } else {
        ("file".to_string(), format!("[文件] {}", filename))
    }
}

/// 获取用户昵称（用于 WebSocket 通知）
async fn get_user_nickname(db: &PgPool, user_id: &str) -> String {
    sqlx::query_scalar::<_, String>(
        r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| user_id.to_string())
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

/// POST /api/storage/upload/confirm
/// 预签名上传完成后调用此接口确认
pub async fn confirm_presigned_upload(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(request): Json<ConfirmUploadRequest>,
) -> Result<Json<FileCompleteResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 确认预签名上传: {}", auth_ctx.user_id, request.file_key);
    
    // 1. 验证并获取文件记录
    let file_record = match state.file_service
        .verify_and_complete_presigned_upload(&request.file_key, &auth_ctx.user_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("验证预签名上传失败: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            ));
        }
    };
    
    let storage_loc: StorageLocation = file_record.storage_location.parse()
        .map_err(|e: String| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))))?;
    
    // 2. 完成上传（创建UUID映射、权限授予）
    let uuid_file_url = match state.file_service
        .complete_presigned_upload(
            &file_record.file_key,
            &auth_ctx.user_id,
            file_record.file_size,
            &file_record.content_type,
            &file_record.preview_support,
            &file_record.storage_location,
            file_record.related_id.as_deref(),
            &file_record.file_hash,
        )
        .await
    {
        Ok(url) => url,
        Err(e) => {
            error!("完成预签名上传失败: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "完成上传失败" })),
            ));
        }
    };
    
    info!("预签名上传确认成功: {}", file_record.file_key);
    let preview_support = file_record.preview_support();
    
    let file_uuid = uuid_file_url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    
    // 3. 好友文件：自动发送消息
    let mut message_uuid: Option<String> = None;
    let mut message_send_time: Option<String> = None;
    
    if storage_loc == StorageLocation::FriendMessages {
        if let Some(friend_id) = &file_record.related_id {
            let (message_type, message_content) = determine_message_type_and_content(
                &file_record.content_type,
                &file_record.file_key,
            );
            
            info!("好友文件上传完成，自动发送 {} 消息给 {}", message_type, friend_id);
            
            match state.message_service.send_message(
                &auth_ctx.user_id,
                friend_id,
                &message_content,
                &message_type,
                Some(file_uuid.clone()),
                Some(uuid_file_url.clone()),
                Some(file_record.file_size),
            ).await {
                Ok((msg_uuid, send_time)) => {
                    info!("自动发送文件消息成功: {}", msg_uuid);
                    message_uuid = Some(msg_uuid.clone());
                    message_send_time = Some(send_time.clone());
                    
                    // 发送 WebSocket 实时通知
                    if let Some(ref notification_service) = state.notification_service {
                        let sender_nickname = get_user_nickname(&state.db, &auth_ctx.user_id).await;
                        let send_time_dt = chrono::DateTime::parse_from_rfc3339(&send_time)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());
                        
                        if let Err(e) = notification_service.notify_friend_message(
                            &auth_ctx.user_id,
                            &sender_nickname,
                            friend_id,
                            &msg_uuid,
                            &message_content,
                            &message_type,
                            send_time_dt,
                        ).await {
                            warn!("好友文件上传 WebSocket 通知失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("自动发送文件消息失败: {}", e);
                }
            }
        }
    }
    
    // 4. 群文件：自动发送群消息
    if storage_loc == StorageLocation::GroupFiles {
        if let Some(group_id_str) = &file_record.related_id {
            if let Ok(group_uuid) = Uuid::parse_str(group_id_str) {
                let (message_type, message_content) = determine_message_type_and_content(
                    &file_record.content_type,
                    &file_record.file_key,
                );
                
                info!("群文件上传完成，自动发送 {} 消息到群 {}", message_type, group_id_str);
                
                match state.group_message_service.send_message(
                    &group_uuid,
                    &auth_ctx.user_id,
                    &message_content,
                    &message_type,
                    Some(&file_uuid),
                    Some(&uuid_file_url),
                    Some(file_record.file_size),
                    None,
                ).await {
                    Ok(resp) => {
                        info!("群文件消息自动发送成功: {}", resp.message_uuid);
                        message_uuid = Some(resp.message_uuid.clone());
                        message_send_time = Some(resp.send_time.clone());
                        
                        // 发送 WebSocket 实时通知
                        if let Some(ref notification_service) = state.notification_service {
                            let sender_nickname = get_user_nickname(&state.db, &auth_ctx.user_id).await;
                            let send_time_dt = chrono::DateTime::parse_from_rfc3339(&resp.send_time)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now());
                            
                            if let Ok(msg_uuid) = Uuid::parse_str(&resp.message_uuid) {
                                if let Err(e) = notification_service.notify_group_message(
                                    &group_uuid,
                                    "",
                                    &auth_ctx.user_id,
                                    &sender_nickname,
                                    &msg_uuid,
                                    &message_content,
                                    &message_type,
                                    send_time_dt,
                                ).await {
                                    warn!("群文件上传 WebSocket 通知失败: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("群文件消息自动发送失败: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(Json(FileCompleteResponse {
        file_url: uuid_file_url,
        file_key: file_record.file_key.clone(),
        file_size: file_record.file_size as u64,
        content_type: file_record.content_type.clone(),
        preview_support,
        message_uuid,
        message_send_time,
    }))
}

