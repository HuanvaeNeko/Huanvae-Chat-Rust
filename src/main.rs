use axum::{routing::get, Router};
use dotenvy::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 导入模块
use huanvae_chat::app_state::AppState;
use huanvae_chat::auth::{handlers::create_auth_routes, utils::KeyManager};
use huanvae_chat::config::get_config;
use huanvae_chat::friends::handlers::create_friend_routes;
use huanvae_chat::friends_messages::handlers::create_messages_routes;
use huanvae_chat::groups::create_group_routes;
use huanvae_chat::group_messages::create_group_messages_routes;
use huanvae_chat::profile::handlers::routes::profile_routes;
use huanvae_chat::storage::{create_storage_routes, S3Client};

/// 配置CORS中间件（从统一配置读取）
fn configure_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{CorsLayer, AllowOrigin};
    use axum::http::{Method, HeaderValue, header};

    let cors_config = &get_config().cors;
    let allowed_origins_str = &cors_config.allowed_origins;

    tracing::info!("🔐 CORS配置: allowed_origins={}", allowed_origins_str);

    let cors = if allowed_origins_str == "*" {
        // 开发环境：允许所有来源
        tracing::warn!("⚠️  警告: CORS配置为允许所有来源，仅适用于开发环境！");
        CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
            ])
            .allow_credentials(false)
    } else {
        // 生产环境：限制特定来源
        let origins: Vec<HeaderValue> = allowed_origins_str
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    match trimmed.parse::<HeaderValue>() {
                        Ok(val) => {
                            tracing::info!("  ✅ 允许来源: {}", trimmed);
                            Some(val)
                        }
                        Err(e) => {
                            tracing::error!("  ❌ 无效的来源 '{}': {}", trimmed, e);
                            None
                        }
                    }
                }
            })
            .collect();

        if origins.is_empty() {
            tracing::warn!("⚠️  未配置有效的CORS来源，将拒绝所有跨域请求");
        }

        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
            ])
            .allow_credentials(true)
            .max_age(Duration::from_secs(cors_config.max_age_secs))
    };

    cors
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载环境变量
    dotenv().ok();

    // 2. 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 HuanVae Chat 启动中...");

    // 3. 获取全局配置
    let config = get_config();

    // 4. 连接数据库（从统一配置读取连接池配置）
    let db_config = &config.database;
    tracing::info!("📊 数据库连接池配置: max={}, min={}, acquire_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        db_config.max_connections, db_config.min_connections, 
        db_config.acquire_timeout_secs, db_config.idle_timeout_secs, db_config.max_lifetime_secs);

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .min_connections(db_config.min_connections)
        .acquire_timeout(Duration::from_secs(db_config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(db_config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(db_config.max_lifetime_secs))
        .connect(&db_config.url)
        .await?;
    tracing::info!("✅ 数据库连接成功");

    // 5. 初始化 MinIO/S3 客户端（从全局配置获取）
    let minio_config = config.minio.clone();
    let s3_client = Arc::new(
        S3Client::new(minio_config)
            .await
            .expect("Failed to initialize S3 client"),
    );
    tracing::info!("✅ MinIO 客户端初始化成功");

    // 6. 加载或生成 RSA 密钥对（从统一配置读取路径）
    let jwt_config = &config.jwt;
    let key_manager = KeyManager::load_or_generate(&jwt_config.private_key_path, &jwt_config.public_key_path)?;

    // 获取API基础URL（从统一配置读取）
    let api_base_url = config.api_base_url.clone();

    // 7. 创建统一应用状态（合并所有服务实例）
    let app_state = AppState::new(db.clone(), key_manager, s3_client.clone(), api_base_url.clone());
    tracing::info!("✅ 应用状态初始化成功");

    // 8. 启动后台定时清理任务（从统一配置读取间隔）
    {
        let blacklist_service = app_state.blacklist_service.clone();
        let cleanup_config = &config.cleanup;
        
        tracing::info!("🧹 定时清理任务已启动:");
        tracing::info!("   - token-blacklist 清理间隔: {}秒", cleanup_config.token_cleanup_interval_secs);
        tracing::info!("   - user-access-cache 清理间隔: {}秒", cleanup_config.cache_cleanup_interval_secs);
        tracing::info!("   - need-blacklist-check 清理间隔: {}秒", cleanup_config.check_cleanup_interval_secs);

        let token_interval_secs = cleanup_config.token_cleanup_interval_secs;
        let cache_interval_secs = cleanup_config.cache_cleanup_interval_secs;
        let check_interval_secs = cleanup_config.check_cleanup_interval_secs;

        tokio::spawn(async move {
            let mut token_interval = tokio::time::interval(Duration::from_secs(token_interval_secs));
            let mut cache_interval = tokio::time::interval(Duration::from_secs(cache_interval_secs));
            let mut check_interval = tokio::time::interval(Duration::from_secs(check_interval_secs));

            loop {
                tokio::select! {
                    _ = token_interval.tick() => {
                        match blacklist_service.cleanup_expired_tokens().await {
                            Ok((total, deleted, remaining)) if deleted > 0 => {
                                tracing::info!("🧹 token-blacklist: 总计 {} 条, 清理 {} 条, 剩余 {} 条", total, deleted, remaining);
                            }
                            Err(e) => {
                                tracing::warn!("清理 token-blacklist 失败: {}", e);
                            }
                            _ => {}
                        }
                    }
                    _ = cache_interval.tick() => {
                        match blacklist_service.cleanup_expired_access_cache().await {
                            Ok((total, deleted, remaining)) if deleted > 0 => {
                                tracing::info!("🧹 user-access-cache: 总计 {} 条, 清理 {} 条, 剩余 {} 条", total, deleted, remaining);
                            }
                            Err(e) => {
                                tracing::warn!("清理 user-access-cache 失败: {}", e);
                            }
                            _ => {}
                        }
                    }
                    _ = check_interval.tick() => {
                        match blacklist_service.cleanup_expired_checks().await {
                            Ok((total, reset, remaining)) if reset > 0 => {
                                tracing::info!("🧹 need-blacklist-check: 总计 {} 个, 重置 {} 个, 剩余 {} 个", total, reset, remaining);
                            }
                            Err(e) => {
                                tracing::warn!("清理 need-blacklist-check 失败: {}", e);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }

    // 9. 创建路由（使用 AppState 生成各模块所需的 State）
    let app = Router::new()
        // 健康检查
        .route("/health", get(|| async { "OK" }))
        .route(
            "/",
            get(|| async { "🚀 HuanVae Chat API is running!\nVersion: 0.1.0" }),
        )
        // 认证路由
        .nest(
            "/api/auth",
            create_auth_routes(
                app_state.register_state(),
                app_state.login_state(),
                app_state.refresh_state(),
                app_state.logout_state(),
                app_state.device_state(),
                app_state.auth_state(),
            ),
        )
        // 好友路由
        .nest(
            "/api/friends",
            create_friend_routes(
                app_state.friends_state(),
                app_state.auth_state(),
                app_state.db.clone(),
            ),
        )
        // 好友消息路由
        .nest(
            "/api/messages",
            create_messages_routes(app_state.messages_state(), app_state.auth_state()),
        )
        // 个人资料路由
        .merge(profile_routes(
            app_state.db.clone(),
            app_state.s3_client.clone(),
            app_state.auth_state(),
            app_state.blacklist_service.clone(),
        ))
        // 文件存储路由
        .nest(
            "/api/storage",
            create_storage_routes(
                app_state.db.clone(),
                app_state.s3_client.clone(),
                app_state.auth_state(),
                api_base_url,
            ),
        )
        // 群聊路由
        .nest(
            "/api/groups",
            create_group_routes(app_state.groups_state(), app_state.auth_state()),
        )
        // 群消息路由
        .nest(
            "/api/group-messages",
            create_group_messages_routes(app_state.group_messages_state(), app_state.auth_state()),
        )
        // CORS 中间件（从环境变量读取配置）
        .layer(configure_cors())
        // 日志中间件
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // 10. 启动服务器（从统一配置读取端口）
    let port = config.server.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    tracing::info!("🌐 服务器监听中: http://0.0.0.0:{}", port);
    tracing::info!("📋 API 端点:");
    tracing::info!("  POST /api/auth/register  - 用户注册");
    tracing::info!("  POST /api/auth/login     - 用户登录");
    tracing::info!("  POST /api/auth/refresh   - 刷新 Token");
    tracing::info!("  POST /api/auth/logout    - 用户登出");
    tracing::info!("  GET  /api/auth/devices   - 查看设备列表");
    tracing::info!("  DELETE /api/auth/devices/:id - 撤销设备");
    tracing::info!("  POST /api/friends/requests         - 提交好友请求");
    tracing::info!("  POST /api/friends/requests/approve - 同意好友请求");
    tracing::info!("  POST /api/friends/requests/reject  - 拒绝好友请求");
    tracing::info!("  GET  /api/friends/requests/sent    - 已发送请求列表");
    tracing::info!("  GET  /api/friends/requests/pending - 待处理请求列表");
    tracing::info!("  GET  /api/friends                  - 已拥有好友列表");
    tracing::info!("  GET  /api/profile                  - 获取个人信息");
    tracing::info!("  PUT  /api/profile                  - 更新个人信息");
    tracing::info!("  PUT  /api/profile/password         - 修改密码");
    tracing::info!("  POST /api/profile/avatar           - 上传头像");
    tracing::info!("  POST /api/messages                 - 发送消息");
    tracing::info!("  GET  /api/messages                 - 获取消息列表");
    tracing::info!("  DELETE /api/messages/delete        - 删除消息");
    tracing::info!("  POST /api/messages/recall          - 撤回消息");
    tracing::info!("  POST /api/storage/upload/request   - 请求文件上传");
    tracing::info!("  POST /api/storage/upload/direct    - 直接上传文件");
    tracing::info!("  GET  /api/storage/multipart/part_url - 获取分片URL");
    tracing::info!("  GET  /api/storage/files             - 查询个人文件列表");
    tracing::info!("  POST /api/groups                    - 创建群聊");
    tracing::info!("  GET  /api/groups/my                 - 获取我的群聊列表");
    tracing::info!("  GET  /api/groups/:id                - 获取群聊信息");
    tracing::info!("  POST /api/groups/:id/invite         - 邀请成员入群");
    tracing::info!("  POST /api/groups/:id/leave          - 退出群聊");
    tracing::info!("  POST /api/groups/:id/transfer       - 转让群主");
    tracing::info!("  POST /api/group-messages            - 发送群消息");
    tracing::info!("  GET  /api/group-messages            - 获取群消息列表");

    axum::serve(listener, app).await?;

    Ok(())
}
