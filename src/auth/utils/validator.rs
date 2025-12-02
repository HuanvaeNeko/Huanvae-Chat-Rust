use crate::common::AppError;
use validator::ValidateEmail;

/// 验证邮箱格式
pub fn validate_email(email: &str) -> Result<(), AppError> {
    if !email.validate_email() {
        return Err(AppError::ValidationError("邮箱格式不正确".to_string()));
    }
    Ok(())
}

/// 验证昵称
/// 要求：2-50个字符
pub fn validate_nickname(nickname: &str) -> Result<(), AppError> {
    let len = nickname.chars().count();
    if len < 2 || len > 50 {
        return Err(AppError::BadRequest(
            "昵称长度必须在2-50个字符之间".to_string(),
        ));
    }
    Ok(())
}

