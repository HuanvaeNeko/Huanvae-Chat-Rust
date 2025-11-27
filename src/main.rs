use axum::{routing::get, Router};
use dotenvy::dotenv;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 导入认证模块
use huanvae_chat::auth::{
    handlers::{
        create_auth_routes, DeviceState, LoginState, LogoutState, RefreshTokenState,
        RegisterState,
    },
    middleware::AuthState,
    services::{BlacklistService, DeviceService, TokenService},
    utils::KeyManager,
};
use huanvae_chat::friends::{
    handlers::create_friend_routes,
    services::FriendsState,
};
use huanvae_chat::friends_messages::{
    handlers::{create_messages_routes, MessagesState},
};
use huanvae_chat::profile::handlers::routes::profile_routes;
use huanvae_chat::storage::{S3Client, S3Config};

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

    // 3. 连接数据库（使用默认连接池配置）
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/huanvae_chat".to_string());

    let db = sqlx::postgres::PgPoolOptions::new()
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

    // 5. 创建服务实例
    let token_service = Arc::new(TokenService::new(key_manager, db.clone()));
    let blacklist_service = Arc::new(BlacklistService::new(db.clone()));
    let device_service = Arc::new(DeviceService::new(db.clone()));

    // 6. 创建状态实例
    let register_state = RegisterState {
        db: db.clone(),
        token_service: token_service.clone(),
    };

    let login_state = LoginState {
        db: db.clone(),
        token_service: token_service.clone(),
    };

    let refresh_state = RefreshTokenState {
        token_service: token_service.clone(),
    };

    let logout_state = LogoutState {
        token_service: token_service.clone(),
        blacklist_service: blacklist_service.clone(),
    };

    let device_state = DeviceState {
        device_service: device_service.clone(),
        blacklist_service: blacklist_service.clone(),
    };

    let auth_state = AuthState {
        token_service: token_service.clone(),
        blacklist_service: blacklist_service.clone(),
        db: db.clone(),
    };

    let friends_state = FriendsState::new(db.clone());
    let messages_state = MessagesState::new(db.clone());

    // 7. 创建路由
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
                register_state,
                login_state,
                refresh_state,
                logout_state,
                device_state,
                auth_state.clone(),
            ),
        )
        .nest(
            "/api/friends",
            create_friend_routes(
                friends_state,
                auth_state.clone(),
                db.clone(),
            ),
        )
        // 好友消息路由
        .nest(
            "/api/messages",
            create_messages_routes(messages_state, auth_state.clone()),
        )
        // 个人资料路由
        .merge(profile_routes(db.clone(), s3_client.clone(), auth_state.clone()))
        // CORS 中间件
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
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

    axum::serve(listener, app).await?;

    Ok(())
}
