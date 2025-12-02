//! 公共模块 - 统一错误类型和响应格式
//!
//! 提供全局统一的：
//! - `AppError` - 统一错误类型
//! - `ApiResponse` - 统一 API 响应格式

mod errors;
mod response;

pub use errors::AppError;
pub use response::ApiResponse;

