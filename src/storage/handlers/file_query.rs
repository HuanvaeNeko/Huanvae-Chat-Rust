use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use tracing::{error, info};

use crate::auth::middleware::AuthContext;
use crate::storage::handlers::upload::StorageState;
use crate::storage::models::{FileListQuery, FileListResponse};

/// GET /api/storage/files - 查询个人文件列表
pub async fn list_files_handler(
    State(state): State<StorageState>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(query): Query<FileListQuery>,
) -> Result<Json<FileListResponse>, (StatusCode, Json<Value>)> {
    info!("用户 {} 查询文件列表", auth_ctx.user_id);
    
    // 1. 参数处理
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let sort_by = query.sort_by.unwrap_or_else(|| "created_at".to_string());
    let sort_order = query.sort_order.unwrap_or_else(|| "desc".to_string());
    
    // 2. 调用服务层查询
    match state.file_service
        .list_user_files(
            &auth_ctx.user_id.to_string(),
            page,
            limit,
            sort_by,
            sort_order,
        )
        .await
    {
        Ok(response) => {
            info!("查询成功，返回 {} 条文件", response.files.len());
            Ok(Json(response))
        }
        Err(e) => {
            error!("查询文件列表失败: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("查询失败: {}", e) })),
            ))
        }
    }
}

