use serde::{Deserialize, Serialize};
use validator::Validate;

/// 更新个人信息请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateProfileRequest {
    /// 新邮箱（可选）
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    
    /// 个性签名（可选）
    #[validate(length(max = 200, message = "Signature too long (max 200 characters)"))]
    pub signature: Option<String>,
}

/// 修改密码请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdatePasswordRequest {
    /// 旧密码
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub old_password: String,
    
    /// 新密码
    #[validate(length(min = 6, max = 100, message = "Password must be 6-100 characters"))]
    pub new_password: String,
}

