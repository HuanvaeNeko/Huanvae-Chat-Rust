use crate::common::AppError;

/// 密码哈希（使用 bcrypt）
pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(AppError::from)
}

/// 验证密码
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash).map_err(AppError::from)
}

/// 验证密码强度
/// 要求：至少8位，包含字母和数字
pub fn validate_password_strength(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::ValidationError(
            "密码格式不符合要求".to_string(),
        ));
    }

    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_letter || !has_digit {
        return Err(AppError::BadRequest(
            "密码必须包含字母和数字".to_string(),
        ));
    }

    Ok(())
}

