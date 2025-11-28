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

/// 创建storage路由
pub fn create_storage_routes(
    db: PgPool,
    s3_client: Arc<S3Client>,
    auth_state: AuthState,
    api_base_url: String,
) -> Router {
    let storage_state = StorageState::new(db, s3_client, api_base_url);

    Router::new()
        // 请求上传（需要鉴权）
        .route("/upload/request", post(request_upload))
        .route("/multipart/part-url", get(get_multipart_part_url))
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            crate::auth::middleware::auth_guard,
        ))
        // 直接上传（Token验证，无需auth_guard）
        // 设置30GB的上传限制
        .route("/upload/direct", post(direct_upload))
        .layer(DefaultBodyLimit::max(30 * 1024 * 1024 * 1024)) // 30GB
        .with_state(storage_state)
}

