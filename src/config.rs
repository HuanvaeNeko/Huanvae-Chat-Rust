//! 应用配置管理
//!
//! 从环境变量加载所有配置项，将硬编码的时间常量等提取为可配置参数

use std::env;

/// Token 相关配置
#[derive(Clone, Debug)]
pub struct TokenConfig {
    /// Access Token 有效期（秒），默认 900 (15分钟)
    pub access_token_ttl: u64,
    /// Refresh Token 有效期（秒），默认 604800 (7天)
    pub refresh_token_ttl: u64,
    /// 黑名单检查窗口（秒），默认 900 (15分钟)
    pub blacklist_check_window: u64,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            access_token_ttl: 900,          // 15分钟
            refresh_token_ttl: 604800,      // 7天
            blacklist_check_window: 900,    // 15分钟
        }
    }
}

impl TokenConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            access_token_ttl: env::var("ACCESS_TOKEN_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(900),
            refresh_token_ttl: env::var("REFRESH_TOKEN_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(604800),
            blacklist_check_window: env::var("BLACKLIST_CHECK_WINDOW_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(900),
        }
    }

    /// Access Token 有效期（分钟）
    pub fn access_token_ttl_minutes(&self) -> i64 {
        (self.access_token_ttl / 60) as i64
    }

    /// Refresh Token 有效期（天）
    pub fn refresh_token_ttl_days(&self) -> i64 {
        (self.refresh_token_ttl / 86400) as i64
    }

    /// 黑名单检查窗口（分钟）
    pub fn blacklist_check_window_minutes(&self) -> i64 {
        (self.blacklist_check_window / 60) as i64
    }
}

/// 存储相关配置
#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// 预签名下载 URL 有效期（秒），默认 300 (5分钟)
    pub presigned_url_ttl: u32,
    /// 分片上传 URL 有效期（秒），默认 3600 (1小时)
    pub multipart_url_ttl: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            presigned_url_ttl: 300,     // 5分钟
            multipart_url_ttl: 3600,    // 1小时
        }
    }
}

impl StorageConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            presigned_url_ttl: env::var("PRESIGNED_URL_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            multipart_url_ttl: env::var("MULTIPART_URL_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
        }
    }
}

/// 消息相关配置
#[derive(Clone, Debug)]
pub struct MessageConfig {
    /// 消息撤回窗口（秒），默认 120 (2分钟)
    pub recall_window: u64,
}

impl Default for MessageConfig {
    fn default() -> Self {
        Self {
            recall_window: 120, // 2分钟
        }
    }
}

impl MessageConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            recall_window: env::var("MESSAGE_RECALL_WINDOW_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
        }
    }
}

/// 服务器配置
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// 服务器端口，默认 8080
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 8080 }
    }
}

impl ServerConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            port: env::var("APP_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
        }
    }
}

/// 数据库配置
#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    /// 数据库连接 URL
    pub url: String,
    /// 最大连接数，默认 20
    pub max_connections: u32,
    /// 最小连接数，默认 5
    pub min_connections: u32,
    /// 获取连接超时（秒），默认 30
    pub acquire_timeout_secs: u64,
    /// 空闲超时（秒），默认 600
    pub idle_timeout_secs: u64,
    /// 连接最大生命周期（秒），默认 1800
    pub max_lifetime_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://postgres:postgres@localhost:5432/huanvae_chat".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        }
    }
}

impl DatabaseConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/huanvae_chat".to_string()),
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            min_connections: env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            acquire_timeout_secs: env::var("DB_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            idle_timeout_secs: env::var("DB_IDLE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
            max_lifetime_secs: env::var("DB_MAX_LIFETIME")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
        }
    }
}

/// JWT 密钥配置
#[derive(Clone, Debug)]
pub struct JwtConfig {
    /// 私钥路径
    pub private_key_path: String,
    /// 公钥路径
    pub public_key_path: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            private_key_path: "./keys/private_key.pem".to_string(),
            public_key_path: "./keys/public_key.pem".to_string(),
        }
    }
}

impl JwtConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            private_key_path: env::var("JWT_PRIVATE_KEY_PATH")
                .unwrap_or_else(|_| "./keys/private_key.pem".to_string()),
            public_key_path: env::var("JWT_PUBLIC_KEY_PATH")
                .unwrap_or_else(|_| "./keys/public_key.pem".to_string()),
        }
    }
}

/// MinIO/S3 存储服务配置
#[derive(Clone, Debug)]
pub struct MinioConfig {
    /// MinIO 服务端点（内部操作用）
    pub endpoint: String,
    /// 预签名URL端点（签名计算用，通过Nginx代理访问）
    pub presign_endpoint: String,
    /// 访问密钥
    pub access_key: String,
    /// 秘密密钥
    pub secret_key: String,
    /// 头像存储桶名称
    pub bucket_avatars: String,
    /// 公开访问 URL
    pub public_url: String,
    /// 区域设置
    pub region: String,
}

impl Default for MinioConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:9000".to_string(),
            presign_endpoint: "http://localhost:9000".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin123".to_string(),
            bucket_avatars: "avatars".to_string(),
            public_url: "http://localhost:9000".to_string(),
            region: "us-east-1".to_string(),
        }
    }
}

impl MinioConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, String> {
        let endpoint = env::var("MINIO_ENDPOINT")
            .map_err(|_| "MINIO_ENDPOINT not set".to_string())?;
        
        Ok(Self {
            presign_endpoint: env::var("MINIO_PRESIGN_ENDPOINT")
                .unwrap_or_else(|_| endpoint.clone()),
            endpoint,
            access_key: env::var("MINIO_ACCESS_KEY")
                .map_err(|_| "MINIO_ACCESS_KEY not set".to_string())?,
            secret_key: env::var("MINIO_SECRET_KEY")
                .map_err(|_| "MINIO_SECRET_KEY not set".to_string())?,
            bucket_avatars: env::var("MINIO_BUCKET_AVATARS")
                .unwrap_or_else(|_| "avatars".to_string()),
            public_url: env::var("MINIO_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            region: env::var("MINIO_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
        })
    }
}

/// 定时清理任务配置
#[derive(Clone, Debug)]
pub struct CleanupConfig {
    /// Token 黑名单清理间隔（秒），默认 3600（每小时）
    pub token_cleanup_interval_secs: u64,
    /// Access Cache 清理间隔（秒），默认 300（每5分钟）
    pub cache_cleanup_interval_secs: u64,
    /// Blacklist Check 清理间隔（秒），默认 60（每分钟）
    pub check_cleanup_interval_secs: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            token_cleanup_interval_secs: 3600,
            cache_cleanup_interval_secs: 300,
            check_cleanup_interval_secs: 60,
        }
    }
}

impl CleanupConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            token_cleanup_interval_secs: env::var("TOKEN_CLEANUP_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            cache_cleanup_interval_secs: env::var("CACHE_CLEANUP_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            check_cleanup_interval_secs: env::var("CHECK_CLEANUP_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
        }
    }
}

/// 应用全局配置
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Token 配置
    pub token: TokenConfig,
    /// 存储配置
    pub storage: StorageConfig,
    /// 消息配置
    pub message: MessageConfig,
    /// API 基础 URL
    pub api_base_url: String,
    /// 服务器配置
    pub server: ServerConfig,
    /// 数据库配置
    pub database: DatabaseConfig,
    /// JWT 配置
    pub jwt: JwtConfig,
    /// 定时清理任务配置
    pub cleanup: CleanupConfig,
    /// MinIO/S3 存储服务配置
    pub minio: MinioConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            token: TokenConfig::default(),
            storage: StorageConfig::default(),
            message: MessageConfig::default(),
            api_base_url: "http://localhost:8080".to_string(),
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            jwt: JwtConfig::default(),
            cleanup: CleanupConfig::default(),
            minio: MinioConfig::default(),
        }
    }
}

impl AppConfig {
    /// 从环境变量加载所有配置
    pub fn from_env() -> Self {
        Self {
            token: TokenConfig::from_env(),
            storage: StorageConfig::from_env(),
            message: MessageConfig::from_env(),
            api_base_url: env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            server: ServerConfig::from_env(),
            database: DatabaseConfig::from_env(),
            jwt: JwtConfig::from_env(),
            cleanup: CleanupConfig::from_env(),
            minio: MinioConfig::from_env().unwrap_or_default(),
        }
    }
}

/// 获取 MinIO 配置
pub fn minio_config() -> &'static MinioConfig {
    &get_config().minio
}

/// 全局配置单例（懒加载）
static CONFIG: std::sync::OnceLock<AppConfig> = std::sync::OnceLock::new();

/// 获取全局配置
pub fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(AppConfig::from_env)
}

/// 获取 Token 配置
pub fn token_config() -> &'static TokenConfig {
    &get_config().token
}

/// 获取存储配置
pub fn storage_config() -> &'static StorageConfig {
    &get_config().storage
}

/// 获取消息配置
pub fn message_config() -> &'static MessageConfig {
    &get_config().message
}

/// 获取服务器配置
pub fn server_config() -> &'static ServerConfig {
    &get_config().server
}

/// 获取数据库配置
pub fn database_config() -> &'static DatabaseConfig {
    &get_config().database
}

/// 获取 JWT 配置
pub fn jwt_config() -> &'static JwtConfig {
    &get_config().jwt
}

/// 获取清理任务配置
pub fn cleanup_config() -> &'static CleanupConfig {
    &get_config().cleanup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.token.access_token_ttl, 900);
        assert_eq!(config.token.refresh_token_ttl, 604800);
        assert_eq!(config.message.recall_window, 120);
    }

    #[test]
    fn test_token_ttl_conversion() {
        let config = TokenConfig::default();
        assert_eq!(config.access_token_ttl_minutes(), 15);
        assert_eq!(config.refresh_token_ttl_days(), 7);
    }
}

