use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// 认证相关错误类型
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("用户名或密码错误")]
    InvalidCredentials,

    #[error("Token 无效")]
    InvalidToken,

    #[error("Token 已过期")]
    TokenExpired,

    #[error("Token 已被撤销")]
    TokenRevoked,

    #[error("Refresh Token 无效")]
    InvalidRefreshToken,

    #[error("用户已存在")]
    UserAlreadyExists,

    #[error("用户不存在")]
    UserNotFound,

    #[error("设备不存在")]
    DeviceNotFound,

    #[error("密码格式不符合要求")]
    InvalidPassword,

    #[error("邮箱格式不正确")]
    InvalidEmail,

    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("加密错误: {0}")]
    CryptoError(String),

    #[error("内部服务器错误")]
    InternalServerError,

    #[error("未授权访问")]
    Unauthorized,

    #[error("权限不足")]
    Forbidden,

    #[error("请求参数错误: {0}")]
    BadRequest(String),
}

/// 将 AuthError 转换为 HTTP 响应
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::TokenRevoked => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidRefreshToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::UserAlreadyExists => (StatusCode::CONFLICT, self.to_string()),
            AuthError::UserNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AuthError::DeviceNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AuthError::InvalidPassword => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误".to_string()),
            AuthError::CryptoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "加密错误".to_string()),
            AuthError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AuthError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

/// 从 sqlx 错误转换
impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        AuthError::DatabaseError(err.to_string())
    }
}

/// 从 bcrypt 错误转换
impl From<bcrypt::BcryptError> for AuthError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AuthError::CryptoError(err.to_string())
    }
}

/// 从 jsonwebtoken 错误转换
impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            ErrorKind::InvalidToken => AuthError::InvalidToken,
            _ => AuthError::InvalidToken,
        }
    }
}

