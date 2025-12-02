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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            token: TokenConfig::default(),
            storage: StorageConfig::default(),
            message: MessageConfig::default(),
            api_base_url: "http://localhost:8080".to_string(),
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
        }
    }
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

