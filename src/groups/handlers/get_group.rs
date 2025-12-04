//! 获取群聊信息处理器

use axum::{extract::{Path, State}, Extension, Json};
use uuid::Uuid;
use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::groups::models::{GroupInfo, GroupListItem};
use super::state::GroupsState;

/// 获取群聊信息
/// GET /api/groups/:group_id
pub async fn get_group_info(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<GroupInfo>>, AppError> {
    // 验证用户是群成员
    if !state.member_service.verify_active_member(&group_id, &auth.user_id).await? {
        return Err(AppError::Forbidden);
    }

    let info = state.group_service.get_group_info(&group_id).await?;
    Ok(Json(ApiResponse::success(info)))
}

/// 获取用户加入的群聊列表
/// GET /api/groups/my
pub async fn get_my_groups(
    State(state): State<GroupsState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<Vec<GroupListItem>>>, AppError> {
    let groups = state.group_service.get_user_groups(&auth.user_id).await?;
    Ok(Json(ApiResponse::success(groups)))
}

/// 搜索群聊（公开信息）
/// GET /api/groups/search?keyword=xxx
pub async fn search_groups(
    State(state): State<GroupsState>,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
) -> Result<Json<ApiResponse<Vec<GroupInfo>>>, AppError> {
    if params.keyword.trim().is_empty() {
        return Err(AppError::BadRequest("搜索关键词不能为空".to_string()));
    }

    // 搜索群聊（只返回活跃的群）
    let groups: Vec<crate::groups::models::Group> = sqlx::query_as(
        r#"SELECT * FROM "groups" 
           WHERE "status" = 'active' 
           AND ("group-name" ILIKE $1 OR "group-id"::text = $2)
           LIMIT 20"#,
    )
    .bind(format!("%{}%", params.keyword))
    .bind(&params.keyword)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("搜索群聊失败: {}", e)))?;

    let infos: Vec<GroupInfo> = groups.into_iter().map(GroupInfo::from).collect();
    Ok(Json(ApiResponse::success(infos)))
}

#[derive(serde::Deserialize)]
pub struct SearchParams {
    pub keyword: String,
}

