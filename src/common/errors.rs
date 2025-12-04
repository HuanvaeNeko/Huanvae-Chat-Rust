//! 统一错误类型
//!
//! 整合所有模块的错误类型，提供统一的错误处理机制

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use super::ApiResponse;

/// 应用统一错误类型
#[derive(Debug, Clone, Error)]
pub enum AppError {
    // ========================================
    // 认证相关错误 (401)
    // ========================================
    #[error("未授权访问")]
    Unauthorized,

    #[error("用户名或密码错误")]
    InvalidCredentials,

    #[error("Token 无效或已过期")]
    InvalidToken,

    #[error("Token 已被撤销")]
    TokenRevoked,

    // ========================================
    // 权限相关错误 (403)
    // ========================================
    #[error("权限不足")]
    Forbidden,

    // ========================================
    // 资源相关错误 (404)
    // ========================================
    #[error("{0}不存在")]
    NotFound(String),

    // ========================================
    // 业务逻辑错误 (400)
    // ========================================
    #[error("{0}")]
    BadRequest(String),

    #[error("验证错误: {0}")]
    ValidationError(String),

    // ========================================
    // 冲突错误 (409)
    // ========================================
    #[error("{0}")]
    Conflict(String),

    // ========================================
    // 服务器内部错误 (500)
    // ========================================
    #[error("内部服务器错误")]
    Internal,

    #[error("数据库错误")]
    Database(String),

    #[error("存储服务错误")]
    Storage(String),
}

impl AppError {
    /// 获取对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 401 Unauthorized
            AppError::Unauthorized
            | AppError::InvalidCredentials
            | AppError::InvalidToken
            | AppError::TokenRevoked => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            AppError::Forbidden => StatusCode::FORBIDDEN,

            // 404 Not Found
            AppError::NotFound(_) => StatusCode::NOT_FOUND,

            // 400 Bad Request
            AppError::BadRequest(_) | AppError::ValidationError(_) => StatusCode::BAD_REQUEST,

            // 409 Conflict
            AppError::Conflict(_) => StatusCode::CONFLICT,

            // 500 Internal Server Error
            AppError::Internal | AppError::Database(_) | AppError::Storage(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// 获取用户可见的错误消息
    /// 对于内部错误，不暴露详细信息
    pub fn user_message(&self) -> String {
        match self {
            // 内部错误不暴露详情
            AppError::Database(_) | AppError::Storage(_) => "内部服务器错误".to_string(),
            // 其他错误直接显示
            _ => self.to_string(),
        }
    }
}

/// 实现 IntoResponse，使 AppError 可以直接作为 Handler 返回值
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.user_message();

        // 记录内部错误详情（不暴露给用户）
        match &self {
            AppError::Database(detail) => {
                tracing::error!("数据库错误: {}", detail);
            }
            AppError::Storage(detail) => {
                tracing::error!("存储服务错误: {}", detail);
            }
            _ => {}
        }

        let body = Json(ApiResponse::<()>::error(status.as_u16(), message));
        (status, body).into_response()
    }
}

// ========================================
// 从其他错误类型转换
// ========================================

/// 从 sqlx::Error 转换
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

/// 从 bcrypt::BcryptError 转换
impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::error!("加密错误: {}", err);
        AppError::Internal
    }
}

/// 从 jsonwebtoken 错误转换
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AppError::InvalidToken,
            ErrorKind::InvalidToken => AppError::InvalidToken,
            _ => AppError::InvalidToken,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(
            AppError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::InvalidCredentials.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(AppError::Forbidden.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            AppError::NotFound("用户".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::BadRequest("错误".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Internal.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_user_message_hides_internal_details() {
        let db_err = AppError::Database("connection refused".to_string());
        assert_eq!(db_err.user_message(), "内部服务器错误");

        let bad_req = AppError::BadRequest("参数错误".to_string());
        assert_eq!(bad_req.user_message(), "参数错误");
    }
}

