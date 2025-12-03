//! 入群申请处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::*;
use super::state::GroupsState;

/// 申请入群（通过搜索）
/// POST /api/groups/:group_id/apply
pub async fn apply_join(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<ApplyJoinRequest>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证群是否存在且活跃
    let group_info = state.group_service.get_group_info(&group_id).await?;
    
    if group_info.status != "active" {
        return Err(AppError::BadRequest("群聊不存在或已解散".to_string()));
    }

    // 检查入群模式
    if group_info.join_mode == "closed" {
        return Err(AppError::BadRequest("群聊已关闭入群".to_string()));
    }
    if group_info.join_mode == "invite_only" || group_info.join_mode == "admin_invite_only" {
        return Err(AppError::BadRequest("该群仅允许通过邀请加入".to_string()));
    }

    // 检查用户是否已在群中
    if state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::BadRequest("您已是该群成员".to_string()));
    }

    // 检查是否已有待处理的申请
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "group-join-requests" 
           WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'pending'"#
    )
    .bind(group_id)
    .bind(&auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    if existing.is_some() {
        return Err(AppError::BadRequest("您已提交过入群申请，请等待审核".to_string()));
    }

    // 根据入群模式处理
    if group_info.join_mode == "open" {
        // 开放模式，直接入群
        state.member_service.add_member(
            &group_id,
            &auth.user_id,
            "search_direct",
            None,
            None,
            None,
        ).await?;
        state.group_service.increment_member_count(&group_id).await?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("已成功加入群聊"))))
    } else {
        // 需要审核
        let request_id = Uuid::now_v7();
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        sqlx::query(
            r#"INSERT INTO "group-join-requests"
               ("id", "group-id", "user-id", "request-type", "message", "status", "created-at", "expires-at")
               VALUES ($1, $2, $3, 'search_apply', $4, 'pending', $5, $6)"#
        )
        .bind(request_id)
        .bind(group_id)
        .bind(&auth.user_id)
        .bind(&req.message)
        .bind(now)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("创建入群申请失败: {}", e);
            AppError::Internal
        })?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("申请已提交，等待管理员审核"))))
    }
}

/// 获取待处理的入群申请（管理员）
/// GET /api/groups/:group_id/requests
pub async fn get_pending_requests(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<JoinRequestListResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let requests: Vec<(Uuid, Uuid, String, String, Option<String>, Option<String>, bool, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"SELECT 
            r."id", r."group-id", r."user-id", r."request-type", 
            r."inviter-id", r."message", r."user-accepted", r."status", r."created-at"
           FROM "group-join-requests" r
           WHERE r."group-id" = $1 AND r."status" = 'pending'
           ORDER BY r."created-at" DESC"#
    )
    .bind(group_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询入群申请失败: {}", e);
        AppError::Internal
    })?;

    let group_info = state.group_service.get_group_info(&group_id).await?;

    let mut result = Vec::new();
    for (id, group_id, user_id, request_type, inviter_id, message, user_accepted, status, created_at) in requests {
        // 获取用户昵称
        let user_nickname: Option<(Option<String>,)> = sqlx::query_as(
            r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#
        )
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        // 获取邀请人昵称
        let inviter_nickname: Option<(Option<String>,)> = if let Some(ref inv_id) = inviter_id {
            sqlx::query_as(
                r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#
            )
            .bind(inv_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        result.push(JoinRequestInfo {
            request_id: id.to_string(),
            group_id: group_id.to_string(),
            group_name: Some(group_info.group_name.clone()),
            user_id,
            user_nickname: user_nickname.and_then(|(n,)| n),
            request_type,
            inviter_id,
            inviter_nickname: inviter_nickname.and_then(|(n,)| n),
            message,
            user_accepted,
            status,
            created_at: created_at.to_rfc3339(),
        });
    }

    Ok(Json(ApiResponse::success(JoinRequestListResponse { requests: result })))
}

/// 同意入群申请
/// POST /api/groups/:group_id/requests/:request_id/approve
pub async fn approve_request(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, request_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    // 查询申请
    let request: (String, String, Option<String>, Option<Uuid>) = sqlx::query_as(
        r#"SELECT "user-id", "request-type", "inviter-id", "invite-code-id"
           FROM "group-join-requests" 
           WHERE "id" = $1 AND "group-id" = $2 AND "status" = 'pending'"#
    )
    .bind(request_id)
    .bind(group_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or_else(|| AppError::BadRequest("申请不存在或已处理".to_string()))?;

    let (user_id, request_type, inviter_id, invite_code_id) = request;

    // 更新申请状态
    let now = chrono::Utc::now();
    sqlx::query(
        r#"UPDATE "group-join-requests" SET 
           "status" = 'approved', 
           "processed-by" = $1, 
           "processed-at" = $2
           WHERE "id" = $3"#
    )
    .bind(&auth.user_id)
    .bind(now)
    .bind(request_id)
    .execute(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    // 确定入群方式
    let join_method = match request_type.as_str() {
        "search_apply" => "search_approved",
        "normal_invite_code" => "normal_invite_code",
        "member_invite" => "member_invite",
        _ => "search_approved",
    };

    // 添加成员
    state.member_service.add_member(
        &group_id,
        &user_id,
        join_method,
        inviter_id.as_deref(),
        Some(&auth.user_id),
        invite_code_id.as_ref(),
    ).await?;
    state.group_service.increment_member_count(&group_id).await?;

    Ok(Json(ApiResponse::success(SuccessResponse::new("已同意入群申请"))))
}

/// 拒绝入群申请
/// POST /api/groups/:group_id/requests/:request_id/reject
pub async fn reject_request(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path((group_id, request_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ProcessJoinRequestBody>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 验证用户是群主或管理员
    if !state.member_service.verify_admin_or_owner(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let now = chrono::Utc::now();
    let result = sqlx::query(
        r#"UPDATE "group-join-requests" SET 
           "status" = 'rejected', 
           "processed-by" = $1, 
           "processed-at" = $2,
           "reject-reason" = $3
           WHERE "id" = $4 AND "group-id" = $5 AND "status" = 'pending'"#
    )
    .bind(&auth.user_id)
    .bind(now)
    .bind(&req.reason)
    .bind(request_id)
    .bind(group_id)
    .execute(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("申请不存在或已处理".to_string()));
    }

    Ok(Json(ApiResponse::success(SuccessResponse::new("已拒绝入群申请"))))
}

/// 获取收到的邀请列表
/// GET /api/groups/invitations
pub async fn get_invitations(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<InvitationListResponse>>, AppError> {
    let invitations: Vec<(Uuid, Uuid, String, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        r#"SELECT 
            r."id", r."group-id", r."inviter-id", r."message", g."group-name",
            r."created-at", r."expires-at"
           FROM "group-join-requests" r
           JOIN "groups" g ON g."group-id" = r."group-id"
           WHERE r."user-id" = $1 
             AND r."status" = 'pending'
             AND r."request-type" IN ('owner_invite', 'admin_invite', 'member_invite')
             AND r."user-accepted" = false
           ORDER BY r."created-at" DESC"#
    )
    .bind(&auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询邀请列表失败: {}", e);
        AppError::Internal
    })?;

    let mut result = Vec::new();
    for (id, group_id, inviter_id, message, group_name, created_at, expires_at) in invitations {
        // 获取群头像
        let group_avatar: Option<(Option<String>,)> = sqlx::query_as(
            r#"SELECT "group-avatar-url" FROM "groups" WHERE "group-id" = $1"#
        )
        .bind(group_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        // 获取邀请人昵称
        let inviter_nickname: Option<(Option<String>,)> = sqlx::query_as(
            r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#
        )
        .bind(&inviter_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        result.push(InvitationInfo {
            request_id: id.to_string(),
            group_id: group_id.to_string(),
            group_name: group_name.unwrap_or_default(),
            group_avatar_url: group_avatar.and_then(|(u,)| u),
            inviter_id,
            inviter_nickname: inviter_nickname.and_then(|(n,)| n),
            message,
            created_at: created_at.to_rfc3339(),
            expires_at: expires_at.map(|t| t.to_rfc3339()),
        });
    }

    Ok(Json(ApiResponse::success(InvitationListResponse { invitations: result })))
}

/// 接受邀请
/// POST /api/groups/invitations/:request_id/accept
pub async fn accept_invitation(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    // 查询邀请
    let request: (Uuid, String, Option<String>, Option<Uuid>) = sqlx::query_as(
        r#"SELECT "group-id", "request-type", "inviter-id", "invite-code-id"
           FROM "group-join-requests" 
           WHERE "id" = $1 AND "user-id" = $2 AND "status" = 'pending'"#
    )
    .bind(request_id)
    .bind(&auth.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or_else(|| AppError::BadRequest("邀请不存在或已处理".to_string()))?;

    let (group_id, request_type, inviter_id, invite_code_id) = request;

    // 验证群是否存在且活跃
    if !state.group_service.verify_group_active(&group_id).await? {
        return Err(AppError::BadRequest("群聊不存在或已解散".to_string()));
    }

    let now = chrono::Utc::now();

    // 如果是群主/管理员邀请，直接入群
    if request_type == "owner_invite" || request_type == "admin_invite" {
        // 更新邀请状态
        sqlx::query(
            r#"UPDATE "group-join-requests" SET 
               "status" = 'approved', 
               "user-accepted" = true,
               "user-accepted-at" = $1,
               "processed-at" = $1
               WHERE "id" = $2"#
        )
        .bind(now)
        .bind(request_id)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

        // 确定入群方式
        let join_method = if request_type == "owner_invite" {
            "owner_invite"
        } else {
            "admin_invite"
        };

        // 添加成员
        state.member_service.add_member(
            &group_id,
            &auth.user_id,
            join_method,
            inviter_id.as_deref(),
            None,
            invite_code_id.as_ref(),
        ).await?;
        state.group_service.increment_member_count(&group_id).await?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("已成功加入群聊"))))
    } else {
        // 普通成员邀请，标记用户已同意，等待管理员审核
        sqlx::query(
            r#"UPDATE "group-join-requests" SET 
               "user-accepted" = true,
               "user-accepted-at" = $1
               WHERE "id" = $2"#
        )
        .bind(now)
        .bind(request_id)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

        Ok(Json(ApiResponse::success(SuccessResponse::new("已同意邀请，等待管理员审核"))))
    }
}

/// 拒绝邀请
/// POST /api/groups/invitations/:request_id/decline
pub async fn decline_invitation(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SuccessResponse>>, AppError> {
    let result = sqlx::query(
        r#"UPDATE "group-join-requests" SET "status" = 'cancelled'
           WHERE "id" = $1 AND "user-id" = $2 AND "status" = 'pending'"#
    )
    .bind(request_id)
    .bind(&auth.user_id)
    .execute(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("邀请不存在或已处理".to_string()));
    }

    Ok(Json(ApiResponse::success(SuccessResponse::new("已拒绝邀请"))))
}

