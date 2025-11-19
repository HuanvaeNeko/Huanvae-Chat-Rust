use crate::auth::errors::AuthError;
use validator::ValidateEmail;

/// 验证邮箱格式
pub fn validate_email(email: &str) -> Result<(), AuthError> {
    if !email.validate_email() {
        return Err(AuthError::InvalidEmail);
    }
    Ok(())
}

/// 验证昵称
/// 要求：2-50个字符
pub fn validate_nickname(nickname: &str) -> Result<(), AuthError> {
    let len = nickname.chars().count();
    if len < 2 || len > 50 {
        return Err(AuthError::BadRequest(
            "昵称长度必须在2-50个字符之间".to_string(),
        ));
    }
    Ok(())
}

