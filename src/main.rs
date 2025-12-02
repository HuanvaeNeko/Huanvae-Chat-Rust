use axum::{routing::get, Router};
use dotenvy::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 导入模块
use huanvae_chat::app_state::AppState;
use huanvae_chat::auth::{handlers::create_auth_routes, utils::KeyManager};
use huanvae_chat::friends::handlers::create_friend_routes;
use huanvae_chat::friends_messages::handlers::create_messages_routes;
use huanvae_chat::profile::handlers::routes::profile_routes;
use huanvae_chat::storage::{create_storage_routes, S3Client, S3Config};

/// 配置CORS中间件（从环境变量读取）
fn configure_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{CorsLayer, AllowOrigin};
    use axum::http::{Method, HeaderValue, header};

    // 读取允许的来源（多个来源用逗号分隔）
    let allowed_origins_str = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "*".to_string());

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
            .max_age(Duration::from_secs(3600))
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

    // 3. 连接数据库（从环境变量读取连接池配置）
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/huanvae_chat".to_string());

    // 连接池配置（从环境变量读取，提供合理默认值）
    let max_connections = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);
    
    let min_connections = std::env::var("DB_MIN_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(5);
    
    let acquire_timeout = std::env::var("DB_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    
    let idle_timeout = std::env::var("DB_IDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(600);
    
    let max_lifetime = std::env::var("DB_MAX_LIFETIME")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800);

    tracing::info!("📊 数据库连接池配置: max={}, min={}, acquire_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        max_connections, min_connections, acquire_timeout, idle_timeout, max_lifetime);

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .idle_timeout(Duration::from_secs(idle_timeout))
        .max_lifetime(Duration::from_secs(max_lifetime))
        .connect(&database_url)
        .await?;
    tracing::info!("✅ 数据库连接成功");

    // 4a. 初始化 MinIO/S3 客户端
    let s3_config = S3Config::from_env().expect("Failed to load MinIO configuration");
    let s3_client = Arc::new(
        S3Client::new(s3_config)
            .await
            .expect("Failed to initialize S3 client"),
    );
    tracing::info!("✅ MinIO 客户端初始化成功");

    // 4. 加载或生成 RSA 密钥对
    let private_key_path = std::env::var("JWT_PRIVATE_KEY_PATH")
        .unwrap_or_else(|_| "./keys/private_key.pem".to_string());
    let public_key_path = std::env::var("JWT_PUBLIC_KEY_PATH")
        .unwrap_or_else(|_| "./keys/public_key.pem".to_string());

    let key_manager = KeyManager::load_or_generate(&private_key_path, &public_key_path)?;

    // 获取API基础URL
    let api_base_url = std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    // 5. 创建统一应用状态（合并所有服务实例）
    let app_state = AppState::new(db.clone(), key_manager, s3_client.clone(), api_base_url.clone());
    tracing::info!("✅ 应用状态初始化成功");

    // 6. 创建路由（使用 AppState 生成各模块所需的 State）
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
        // CORS 中间件（从环境变量读取配置）
        .layer(configure_cors())
        // 日志中间件
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // 8. 启动服务器
    let port = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

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
    tracing::info!("  GET  /api/storage/multipart/part-url - 获取分片URL");
    tracing::info!("  GET  /api/storage/files             - 查询个人文件列表");

    axum::serve(listener, app).await?;

    Ok(())
}
