use crate::auth::middleware::{auth_guard, AuthState};
use crate::storage::S3Client;
use axum::{
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
    pub pool: PgPool,
    pub s3_client: Arc<S3Client>,
}

/// 配置 profile 路由
pub fn profile_routes(pool: PgPool, s3_client: Arc<S3Client>, auth_state: AuthState) -> Router {
    let state = ProfileAppState {
        pool,
        s3_client,
    };

    Router::new()
        .route("/api/profile", get(get_profile))
        .route("/api/profile", put(update_profile))
        .route("/api/profile/password", put(update_password))
        .route("/api/profile/avatar", post(upload_avatar))
        .layer(middleware::from_fn_with_state(auth_state, auth_guard))
        .with_state(state)
}

