use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::auth::middleware::AuthState;
use crate::storage::client::S3Client;
use crate::storage::handlers::upload::*;
use crate::storage::handlers::file_access::*;
use crate::storage::services::UuidMappingService;

/// 创建storage路由
pub fn create_storage_routes(
    db: PgPool,
    s3_client: Arc<S3Client>,
    auth_state: AuthState,
    api_base_url: String,
) -> Router {
    let storage_state = StorageState::new(db.clone(), s3_client.clone(), api_base_url);
    
    // 创建文件访问状态
    let file_access_state = FileAccessState {
        uuid_mapping_service: Arc::new(UuidMappingService::new(db)),
        s3_client,
    };

    // 上传相关路由
    let upload_router = Router::new()
        .route("/upload/request", post(request_upload))
        .route("/multipart/part-url", get(get_multipart_part_url))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::middleware::auth_guard,
        ))
        .route("/upload/direct", post(direct_upload))
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024 * 1024)) // 30GB
        .with_state(storage_state);
    
    // 文件访问路由
    let file_access_router = Router::new()
        .route("/file/{uuid}", get(get_file_by_uuid))
        .route_layer(middleware::from_fn_with_state(
            auth_state,
            crate::auth::middleware::auth_guard,
        ))
        .with_state(file_access_state);
    
    // 合并路由
    upload_router.merge(file_access_router)
}

