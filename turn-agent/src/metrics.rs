//! 指标采集模块
//!
//! 采集节点运行时指标

use std::process::Command;
use std::sync::Mutex;
use sysinfo::System;

use crate::protocol::NodeMetrics;

/// 指标采集器
pub struct MetricsCollector {
    system: Mutex<System>,
}

impl MetricsCollector {
    /// 创建指标采集器
    pub fn new() -> Self {
        Self {
            system: Mutex::new(System::new_all()),
        }
    }

    /// 采集当前指标
    pub async fn collect(&self) -> NodeMetrics {
        let mut sys = self.system.lock().unwrap();
        sys.refresh_all();

        // CPU 使用率
        let cpu_percent = sys.global_cpu_usage();

        // 内存使用率
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let memory_percent = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };

        // 活跃会话数
        let active_sessions = self.get_coturn_sessions().unwrap_or(0);

        // 带宽使用（简化实现）
        let (bandwidth_in, bandwidth_out) = self.get_bandwidth();

        // 运行时间
        let uptime_seconds = System::uptime();

        NodeMetrics {
            active_sessions,
            total_allocations: 0, // TODO: 从 coturn 统计获取
            bandwidth_in_mbps: bandwidth_in,
            bandwidth_out_mbps: bandwidth_out,
            cpu_percent,
            memory_percent,
            uptime_seconds,
        }
    }

    /// 获取 Coturn 活跃会话数
    fn get_coturn_sessions(&self) -> Option<u32> {
        // 方法1: 统计 TURN 端口的 ESTABLISHED 连接数
        let output = Command::new("sh")
            .args([
                "-c",
                "netstat -an 2>/dev/null | grep ':3478' | grep -c ESTABLISHED || echo 0",
            ])
            .output()
            .ok()?;

        let count_str = String::from_utf8_lossy(&output.stdout);
        count_str.trim().parse().ok()
    }

    /// 获取带宽使用
    ///
    /// 返回 (入站 Mbps, 出站 Mbps)
    fn get_bandwidth(&self) -> (f64, f64) {
        // 简化实现：读取 /proc/net/dev
        // 实际生产环境可以使用更精确的方法
        
        // 暂时返回 0，后续可以实现
        (0.0, 0.0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

