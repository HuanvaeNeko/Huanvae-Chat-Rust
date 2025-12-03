use crate::common::AppError;
use crate::storage::models::{FileType, PreviewSupport, UploadMode};

/// 文件验证服务
pub struct FileValidator;

impl FileValidator {
    /// 判断上传模式
    pub fn determine_upload_mode(file_size: u64) -> UploadMode {
        if file_size <= 15 * 1024 * 1024 * 1024 {  // 15GB
            UploadMode::OneTimeToken
        } else {
            UploadMode::PresignedUrl
        }
    }

    /// 判断文件类型和预览支持
    pub fn determine_file_type_and_preview(
        content_type: &str,
        file_size: u64,
        is_friend_message: bool,
    ) -> Result<(FileType, PreviewSupport), AppError> {
        // 图片处理
        if content_type.starts_with("image/") {
            if file_size <= 100 * 1024 * 1024 {  // 100MB
                let file_type = if is_friend_message {
                    FileType::FriendImage
                } else {
                    FileType::UserImage
                };
                Ok((file_type, PreviewSupport::InlinePreview))
            } else if file_size <= 15 * 1024 * 1024 * 1024 {  // 15GB
                let file_type = if is_friend_message {
                    FileType::FriendImageFile
                } else {
                    FileType::UserImageFile
                };
                Ok((file_type, PreviewSupport::DownloadOnly))
            } else {
                Err(AppError::BadRequest("图片大小超过15GB，请使用超大文件上传".to_string()))
            }
        }
        // 视频处理
        else if content_type.starts_with("video/") {
            if file_size <= 15 * 1024 * 1024 * 1024 {  // 15GB
                let file_type = if is_friend_message {
                    FileType::FriendVideo
                } else {
                    FileType::UserVideo
                };
                Ok((file_type, PreviewSupport::InlinePreview))
            } else if file_size <= 30 * 1024 * 1024 * 1024 {  // 30GB
                let file_type = if is_friend_message {
                    FileType::FriendVideoFile
                } else {
                    FileType::UserVideoFile
                };
                Ok((file_type, PreviewSupport::DownloadOnly))
            } else {
                Err(AppError::BadRequest("视频大小超过30GB，请联系管理员使用超大文件上传流程".to_string()))
            }
        }
        // 普通文档
        else {
            if file_size <= 15 * 1024 * 1024 * 1024 {  // 15GB
                let file_type = if is_friend_message {
                    FileType::FriendDocument
                } else {
                    FileType::UserDocument
                };
                Ok((file_type, PreviewSupport::DownloadOnly))
            } else if file_size <= 30 * 1024 * 1024 * 1024 {  // 30GB
                Ok((FileType::UserDocument, PreviewSupport::DownloadOnly))
            } else {
                Err(AppError::BadRequest("文件大小超过30GB，请联系管理员使用超大文件上传流程".to_string()))
            }
        }
    }

    /// 验证哈希格式
    pub fn validate_hash(hash: &str) -> Result<(), AppError> {
        if hash.len() != 64 {
            return Err(AppError::ValidationError("哈希值必须是64位十六进制字符串（SHA-256）".to_string()));
        }
        
        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::ValidationError("哈希值包含非法字符".to_string()));
        }
        
        Ok(())
    }

    /// 获取最大文件大小限制
    pub fn get_max_file_size(file_type: &FileType) -> u64 {
        match file_type {
            FileType::Avatar => 5 * 1024 * 1024,              // 5MB
            FileType::UserImage | FileType::FriendImage => 100 * 1024 * 1024,  // 100MB
            FileType::UserImageFile | FileType::FriendImageFile => 15 * 1024 * 1024 * 1024,  // 15GB
            FileType::UserVideo | FileType::FriendVideo => 15 * 1024 * 1024 * 1024,  // 15GB
            FileType::UserVideoFile | FileType::FriendVideoFile => 30 * 1024 * 1024 * 1024,  // 30GB
            FileType::UserDocument | FileType::FriendDocument => 15 * 1024 * 1024 * 1024,  // 15GB
            _ => 30 * 1024 * 1024 * 1024,  // 30GB
        }
    }

    /// 验证文件类型
    pub fn validate_file_type(
        file_type: &FileType,
        content_type: &str,
    ) -> Result<(), AppError> {
        match file_type {
            FileType::Avatar | FileType::UserImage | FileType::UserImageFile |
            FileType::FriendImage | FileType::FriendImageFile | FileType::GroupImage => {
                // 图片类型 - 添加更多格式支持
                if !["image/jpeg", "image/png", "image/gif", "image/webp", "image/jpg", "image/tiff", "image/tif"].contains(&content_type) {
                    return Err(AppError::ValidationError("不支持的图片格式，仅支持：jpg、png、gif、webp、tiff".to_string()));
                }
            }
            FileType::UserVideo | FileType::UserVideoFile |
            FileType::FriendVideo | FileType::FriendVideoFile | FileType::GroupVideo => {
                if !content_type.starts_with("video/") {
                    return Err(AppError::ValidationError("不支持的视频格式".to_string()));
                }
            }
            _ => {} // 文档类型不限制
        }
        Ok(())
    }

    /// 验证文件大小
    pub fn validate_file_size(
        file_type: &FileType,
        size: u64,
    ) -> Result<(), AppError> {
        let max_size = Self::get_max_file_size(file_type);
        
        if size > max_size {
            return Err(AppError::ValidationError(format!(
                "文件大小超过限制: 最大 {} MB",
                max_size / 1024 / 1024
            )));
        }
        Ok(())
    }

    /// 计算推荐的有效期（根据文件大小）
    pub fn calculate_expires_in(file_size: u64, user_specified: Option<u32>) -> u32 {
        if let Some(expires) = user_specified {
            return expires.clamp(300, 604800); // 5分钟到7天
        }

        match file_size {
            0..=10_485_760 => 300,                    // < 10MB: 5分钟
            10_485_761..=104_857_600 => 900,          // 10-100MB: 15分钟
            104_857_601..=1_073_741_824 => 1800,      // 100MB-1GB: 30分钟
            1_073_741_825..=10_737_418_240 => 7200,   // 1-10GB: 2小时
            _ => 14400,                                // 10-15GB: 4小时
        }
    }

    /// 获取文件扩展名
    pub fn get_extension(filename: &str) -> String {
        filename
            .rsplit('.')
            .next()
            .unwrap_or("bin")
            .to_lowercase()
    }

    /// 清理文件名（移除特殊字符）
    pub fn sanitize_filename(filename: &str) -> String {
        filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .take(100)
            .collect()
    }
}

/// 计算SHA-256哈希
pub fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

