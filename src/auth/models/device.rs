use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// 设备信息模型（用于多设备登录管理）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// 设备ID
    pub device_id: String,
    
    /// 设备信息（操作系统、浏览器等）
    pub device_info: String,
    
    /// IP地址
    pub ip_address: Option<String>,
    
    /// 最后活跃时间
    pub last_used_at: Option<NaiveDateTime>,
    
    /// 创建时间
    pub created_at: NaiveDateTime,
    
    /// 是否是当前设备
    pub is_current: bool,
}

/// 设备列表响应
#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    pub devices: Vec<Device>,
    pub total: usize,
}

/// 撤销设备请求
#[derive(Debug, Deserialize)]
pub struct RevokeDeviceRequest {
    pub device_id: String,
}

