//! 更新群聊信息处理器

use axum::{extract::{Multipart, Path, State}, Extension, Json};
use uuid::Uuid;
use tracing::{error, info};
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{UpdateGroupRequest, UpdateJoinModeRequest, UpdateNicknameRequest, SuccessResponse, AvatarResponse};
use crate::storage::services::AvatarService;
use super::state::GroupsState;

/// 更新群聊信息
/// PUT /api/groups/:group_id
pub async fn update_group_info(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.group_service.update_group_info(
        &group_id,
        req.group_name.as_deref(),
        req.group_description.as_deref(),
    ).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("群信息更新成功"))))
}

/// 修改入群模式
/// PUT /api/groups/:group_id/join-mode
pub async fn update_join_mode(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdateJoinModeRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 只有群主可以修改入群模式
    if !state.member_service.verify_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    state.group_service.update_join_mode(&group_id, &req.join_mode).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("入群模式更新成功"))))
}

/// 上传群头像
/// POST /api/groups/:group_id/avatar
pub async fn upload_group_avatar(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<AvatarResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

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
                    error!("读取群头像文件失败: {}", e);
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
    let group_key = format!("group-{}", group_id);
    info!("上传群头像: group_id={}, filename={}", group_id, fname);

    // 上传到 MinIO（复用 AvatarService）
    let avatar_url = AvatarService::upload_avatar(&state.s3_client, &group_key, data, &fname)
        .await
        .map_err(|e| {
            error!("上传群头像失败: {}", e);
            AppError::Storage(e.to_string())
        })?;

    // 更新数据库
    state.group_service.update_group_avatar(&group_id, &avatar_url).await?;

    Ok(Json(ApiResponse::success(AvatarResponse {
        avatar_url,
        message: "群头像上传成功".to_string(),
    })))
}

/// 修改群内昵称
/// PUT /api/groups/:group_id/nickname
pub async fn update_member_nickname(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdateNicknameRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 更新昵称（方法内部会验证是否为群成员）
    state.member_service.update_nickname(
        &group_id,
        &auth.user_id,
        req.nickname.as_deref(),
    ).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("群内昵称更新成功"))))
}

