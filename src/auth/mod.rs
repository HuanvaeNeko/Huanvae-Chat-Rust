// 认证模块 - 完整的 JWT 多设备认证系统
//
// 架构说明：
// - models: 数据模型（User, Claims, RefreshToken, Device）
// - utils: 工具函数（密钥管理、密码哈希、验证器）
// - services: 业务逻辑（Token服务、黑名单服务、设备服务）
// - middleware: 鉴权中间件（验证 Access Token + 智能黑名单检查）
// - handlers: HTTP 请求处理（注册、登录、刷新、登出、设备管理）
//
// 错误处理：统一使用 crate::common::AppError

pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

// 导出常用类型
pub use middleware::{auth_guard, AuthContext, AuthState};
pub use models::{
    AccessTokenClaims, Device, LoginRequest, RefreshTokenClaims, RegisterRequest, TokenResponse,
    User, UserResponse,
};
pub use services::{BlacklistService, DeviceService, TokenService};
pub use utils::{hash_password, verify_password, KeyManager};

