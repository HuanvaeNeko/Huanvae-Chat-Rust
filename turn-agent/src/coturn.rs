//! Coturn 管理模块
//!
//! 管理 Coturn 配置文件生成和进程控制

use chrono::Utc;
use std::process::Command;
use tokio::fs;
use tokio::sync::RwLock;

use crate::config::AgentConfig;
use crate::protocol::TurnConfig;

/// Coturn 管理器
pub struct CoturnManager {
    /// 配置文件路径
    config_path: String,
    /// 配置模板路径
    template_path: String,
    /// 当前密钥
    current_secret: RwLock<String>,
    /// 当前配置版本
    config_version: RwLock<u64>,
}

/// Coturn 管理错误
#[derive(Debug, thiserror::Error)]
pub enum CoturnError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("模板读取失败: {0}")]
    TemplateRead(String),

    #[error("配置写入失败: {0}")]
    ConfigWrite(String),
}

impl CoturnManager {
    /// 创建 Coturn 管理器
    pub fn new(config_path: String, template_path: String) -> Self {
        Self {
            config_path,
            template_path,
            current_secret: RwLock::new(String::new()),
            config_version: RwLock::new(0),
        }
    }

    /// 应用配置
    ///
    /// 从模板生成配置文件并写入
    pub async fn apply_config(
        &self,
        turn_config: &TurnConfig,
        agent_config: &AgentConfig,
        version: u64,
    ) -> Result<(), CoturnError> {
        // 读取模板
        let template = fs::read_to_string(&self.template_path)
            .await
            .map_err(|e| CoturnError::TemplateRead(e.to_string()))?;

        // 替换变量
        let now = Utc::now().to_rfc3339();
        let config_content = template
            .replace("${GENERATED_AT}", &now)
            .replace("${CONFIG_VERSION}", &version.to_string())
            .replace("${TURN_PORT}", &agent_config.turn_port.to_string())
            .replace("${TURN_TLS_PORT}", &agent_config.turn_tls_port.to_string())
            .replace("${PUBLIC_IP}", &agent_config.public_ip)
            .replace("${AUTH_SECRET}", &turn_config.auth_secret)
            .replace("${REALM}", &turn_config.realm)
            .replace("${RELAY_MIN_PORT}", &agent_config.relay_min_port.to_string())
            .replace("${RELAY_MAX_PORT}", &agent_config.relay_max_port.to_string())
            .replace("${TOTAL_QUOTA}", &turn_config.total_quota.to_string())
            .replace("${USER_QUOTA}", &turn_config.user_quota.to_string())
            .replace("${MAX_BPS}", &turn_config.max_bps.to_string());

        // 确保目录存在
        if let Some(parent) = std::path::Path::new(&self.config_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        // 写入配置文件
        fs::write(&self.config_path, config_content)
            .await
            .map_err(|e| CoturnError::ConfigWrite(e.to_string()))?;

        // 保存当前密钥和版本
        *self.current_secret.write().await = turn_config.auth_secret.clone();
        *self.config_version.write().await = version;

        tracing::info!(
            "配置已写入: {} (版本: {})",
            self.config_path,
            version
        );

        Ok(())
    }

    /// 仅更新密钥
    ///
    /// 不重新生成整个配置，只替换密钥行
    pub async fn update_secret(&self, secret: &str) -> Result<(), CoturnError> {
        // 读取当前配置
        let content = fs::read_to_string(&self.config_path).await?;

        // 替换密钥行
        let old_secret = self.current_secret.read().await;
        let updated = if old_secret.is_empty() {
            // 如果旧密钥为空，直接替换占位符
            content.replace("${AUTH_SECRET}", secret)
        } else {
            // 替换旧密钥
            content.replace(
                &format!("static-auth-secret={}", *old_secret),
                &format!("static-auth-secret={}", secret),
            )
        };

        // 写回
        fs::write(&self.config_path, updated).await?;

        // 更新内存中的密钥
        drop(old_secret);
        *self.current_secret.write().await = secret.to_string();

        tracing::info!("密钥已更新");

        Ok(())
    }

    /// 重载 Coturn 配置
    ///
    /// 向 Coturn 进程发送 SIGHUP 信号
    pub async fn reload(&self) -> Result<(), CoturnError> {
        // 方法1: 使用 pkill 发送 SIGHUP
        let result = Command::new("pkill")
            .args(["-HUP", "turnserver"])
            .status();

        match result {
            Ok(status) if status.success() => {
                tracing::info!("Coturn 重载信号已发送");
                Ok(())
            }
            Ok(status) => {
                // 进程可能不存在，这在首次启动时是正常的
                tracing::debug!("pkill 退出码: {}", status);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("发送重载信号失败: {}", e);
                // 不作为致命错误
                Ok(())
            }
        }
    }

    /// 检查配置文件是否存在
    #[allow(dead_code)]
    pub async fn config_exists(&self) -> bool {
        fs::metadata(&self.config_path).await.is_ok()
    }

    /// 获取当前配置版本
    #[allow(dead_code)]
    pub async fn get_config_version(&self) -> u64 {
        *self.config_version.read().await
    }
}

