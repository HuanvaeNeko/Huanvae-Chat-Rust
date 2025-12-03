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
use crate::storage::handlers::file_access::{
    generate_presigned_url, generate_extended_presigned_url,
};
use crate::storage::handlers::friends_file_access::{
    generate_friend_file_presigned_url, generate_friend_file_extended_presigned_url,
};
use crate::storage::handlers::file_query::list_files_handler;

/// 创建storage路由
pub fn create_storage_routes(
    db: PgPool,
    s3_client: Arc<S3Client>,
    auth_state: AuthState,
    api_base_url: String,
) -> Router {
    let storage_state = StorageState::new(db.clone(), s3_client.clone(), api_base_url);

    // 上传相关路由
    let upload_router = Router::new()
        .route("/upload/request", post(request_upload))
        .route("/multipart/part_url", get(get_multipart_part_url))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::middleware::auth_guard,
        ))
        .route("/upload/direct", post(direct_upload))
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024 * 1024)) // 30GB
        .with_state(storage_state.clone());
    
    // 预签名URL路由（个人文件）
    let presigned_router = Router::new()
        .route("/file/{uuid}/presigned_url", post(generate_presigned_url))
        .route("/file/{uuid}/presigned_url/extended", post(generate_extended_presigned_url))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::middleware::auth_guard,
        ))
        .with_state(storage_state.clone());
    
    // 好友文件预签名URL路由
    let friends_file_router = Router::new()
        .route("/friends-file/{uuid}/presigned-url", post(generate_friend_file_presigned_url))
        .route("/friends-file/{uuid}/presigned-url/extended", post(generate_friend_file_extended_presigned_url))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::middleware::auth_guard,
        ))
        .with_state(storage_state.clone());
    
    // 文件查询路由
    let query_router = Router::new()
        .route("/files", get(list_files_handler))
        .route_layer(middleware::from_fn_with_state(
            auth_state,
            crate::auth::middleware::auth_guard,
        ))
        .with_state(storage_state);
    
    // 合并路由
    upload_router
        .merge(presigned_router)
        .merge(friends_file_router)
        .merge(query_router)
}
