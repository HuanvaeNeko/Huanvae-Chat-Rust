use crate::auth::middleware::{auth_guard, AuthState};
use crate::auth::services::BlacklistService;
use crate::profile::services::ProfileService;
use crate::storage::S3Client;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post, put},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

use super::{get_profile, update_password, update_profile, upload_avatar};

/// Profile 模块的应用状态
#[derive(Clone)]
pub struct ProfileAppState {
    pub profile_service: ProfileService,
    pub s3_client: Arc<S3Client>,
    pub blacklist_service: Arc<BlacklistService>,
}

/// 配置 profile 路由
pub fn profile_routes(
    pool: PgPool,
    s3_client: Arc<S3Client>,
    auth_state: AuthState,
    blacklist_service: Arc<BlacklistService>,
) -> Router {
    let state = ProfileAppState {
        profile_service: ProfileService::new(pool),
        s3_client,
        blacklist_service,
    };

    Router::new()
        .route("/api/profile", get(get_profile))
        .route("/api/profile", put(update_profile))
        .route("/api/profile/password", put(update_password))
        .route("/api/profile/avatar", post(upload_avatar))
        .layer(DefaultBodyLimit::max(15 * 1024 * 1024)) // 15MB 限制（头像最大10MB + 余量）
        .layer(middleware::from_fn_with_state(auth_state, auth_guard))
        .with_state(state)
}

