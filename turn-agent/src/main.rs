//! Turn Agent 主程序
//!
//! 连接主服务器，接收配置，管理 Coturn

use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

mod config;
mod coordinator;
mod coturn;
mod metrics;
mod protocol;

use config::AgentConfig;
use coordinator::CoordinatorClient;
use coturn::CoturnManager;
use metrics::MetricsCollector;
use protocol::{AgentMessage, CoordinatorMessage, NodeCapabilities, NodeCommand, TurnPorts};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("turn_agent=info".parse().unwrap())
                .add_directive("tokio_tungstenite=warn".parse().unwrap()),
        )
        .init();

    info!("======================================");
    info!("  Turn Agent 启动中...");
    info!("======================================");

    // 加载配置
    let config = match AgentConfig::load() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("配置加载失败: {}", e);
            error!("请检查 .env 文件是否正确配置");
            std::process::exit(1);
        }
    };

    info!("配置加载成功:");
    info!("  节点 ID:   {}", config.node_id);
    info!("  区域:      {}", config.region);
    info!("  公网 IP:   {}", config.public_ip);
    info!("  服务器:    {}", config.coordinator_url);
    info!("  心跳间隔:  {}秒", config.heartbeat_interval);

    // 初始化 Coturn 管理器
    // 检测运行环境，使用不同的路径
    let (config_path, template_path) = if std::path::Path::new("/app/templates").exists() {
        // Docker 容器内
        (
            "/etc/turnserver/turnserver.conf".to_string(),
            "/app/templates/turnserver.conf.template".to_string(),
        )
    } else {
        // 本地开发环境
        (
            "./data/config/turnserver.conf".to_string(),
            "./config/turnserver.conf.template".to_string(),
        )
    };
    let coturn_manager = Arc::new(CoturnManager::new(config_path, template_path));

    // 初始化指标采集器
    let metrics_collector = Arc::new(MetricsCollector::new());

    // 主循环：断开后自动重连
    loop {
        info!("正在连接主服务器...");

        match run_connection(
            config.clone(),
            coturn_manager.clone(),
            metrics_collector.clone(),
        )
        .await
        {
            Ok(_) => {
                info!("与主服务器断开连接");
            }
            Err(e) => {
                error!("连接错误: {}", e);
            }
        }

        warn!("5 秒后重新连接...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// 运行连接
async fn run_connection(
    config: Arc<AgentConfig>,
    coturn_manager: Arc<CoturnManager>,
    metrics_collector: Arc<MetricsCollector>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 建立 WebSocket 连接
    let client = CoordinatorClient::connect(
        &config.coordinator_url,
        &config.coordinator_token,
    )
    .await?;

    info!("已连接到主服务器");

    // 发送注册消息
    let register_msg = AgentMessage::Register {
        node_id: config.node_id.clone(),
        region: config.region.clone(),
        public_ip: config.public_ip.clone(),
        ports: TurnPorts {
            listening: config.turn_port,
            tls: config.turn_tls_port,
            min_relay: config.relay_min_port,
            max_relay: config.relay_max_port,
        },
        capabilities: NodeCapabilities {
            supports_tcp: true,
            supports_tls: true,
            supports_dtls: true,
            max_bandwidth_mbps: 1000,
        },
    };

    client.send(&register_msg).await?;
    info!("注册消息已发送");

    // 启动心跳任务
    let client_heartbeat = client.clone();
    let metrics_heartbeat = metrics_collector.clone();
    let heartbeat_interval_secs = config.heartbeat_interval;

    let heartbeat_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(heartbeat_interval_secs));

        loop {
            ticker.tick().await;

            let metrics = metrics_heartbeat.collect().await;
            let msg = AgentMessage::Heartbeat { metrics };

            if let Err(e) = client_heartbeat.send(&msg).await {
                warn!("心跳发送失败: {}", e);
                break;
            }
        }
    });

    // 消息处理循环
    let config_for_loop = config.clone();
    while let Some(msg) = client.recv().await? {
        match msg {
            CoordinatorMessage::Registered { node_id, assigned_id } => {
                let final_id = assigned_id.as_ref().unwrap_or(&node_id);
                info!("注册成功: {}", final_id);
            }

            CoordinatorMessage::Config { version, config: turn_config } => {
                info!("收到配置 (版本: {})", version);

                // 应用配置
                match coturn_manager
                    .apply_config(&turn_config, &config_for_loop, version)
                    .await
                {
                    Ok(_) => {
                        info!("配置应用成功");

                        // 重载 coturn
                        if let Err(e) = coturn_manager.reload().await {
                            error!("Coturn 重载失败: {}", e);
                            client
                                .send(&AgentMessage::ConfigApplied {
                                    config_version: version,
                                    success: false,
                                    error: Some(e.to_string()),
                                })
                                .await?;
                        } else {
                            client
                                .send(&AgentMessage::ConfigApplied {
                                    config_version: version,
                                    success: true,
                                    error: None,
                                })
                                .await?;
                        }
                    }
                    Err(e) => {
                        error!("配置应用失败: {}", e);
                        client
                            .send(&AgentMessage::ConfigApplied {
                                config_version: version,
                                success: false,
                                error: Some(e.to_string()),
                            })
                            .await?;
                    }
                }
            }

            CoordinatorMessage::UpdateSecret {
                secret,
                effective_at,
                expires_at,
            } => {
                info!(
                    "收到密钥更新 (生效: {}, 过期: {})",
                    effective_at, expires_at
                );

                if let Err(e) = coturn_manager.update_secret(&secret).await {
                    error!("密钥更新失败: {}", e);
                } else {
                    let _ = coturn_manager.reload().await;
                }
            }

            CoordinatorMessage::Command { command } => {
                info!("收到命令: {:?}", command);

                match command {
                    NodeCommand::Reload => {
                        let _ = coturn_manager.reload().await;
                    }
                    NodeCommand::Shutdown => {
                        info!("收到关闭命令");
                        break;
                    }
                    NodeCommand::DrainAndShutdown => {
                        info!("收到排空关闭命令");
                        // TODO: 等待现有会话结束
                        break;
                    }
                }
            }

            CoordinatorMessage::Error { code, message } => {
                error!("服务器错误: {} - {}", code, message);
            }
        }
    }

    heartbeat_handle.abort();
    Ok(())
}
