//! 统一 API 响应格式

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

/// 统一 API 响应结构
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 请求是否成功
    pub success: bool,
    /// HTTP 状态码
    pub code: u16,
    /// 响应数据（成功时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 成功消息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 错误消息（失败时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应（带数据）
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            code: 200,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    /// 创建成功响应（带数据和消息）
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            code: 200,
            data: Some(data),
            message: Some(message.into()),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    /// 创建成功响应（仅消息，无数据）
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            code: 200,
            data: None,
            message: Some(message.into()),
            error: None,
        }
    }

    /// 创建错误响应
    pub fn error(code: u16, error: impl Into<String>) -> Self {
        Self {
            success: false,
            code,
            data: None,
            message: None,
            error: Some(error.into()),
        }
    }

    /// 创建错误响应（带 StatusCode）
    pub fn from_error(status: StatusCode, error: impl Into<String>) -> Self {
        Self::error(status.as_u16(), error)
    }
}

/// 实现 IntoResponse，使 ApiResponse 可以直接作为 Handler 返回值
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let resp = ApiResponse::success("test data");
        assert!(resp.success);
        assert_eq!(resp.code, 200);
        assert_eq!(resp.data, Some("test data"));
    }

    #[test]
    fn test_error_response() {
        let resp = ApiResponse::error(400, "Bad request");
        assert!(!resp.success);
        assert_eq!(resp.code, 400);
        assert_eq!(resp.error, Some("Bad request".to_string()));
    }
}

