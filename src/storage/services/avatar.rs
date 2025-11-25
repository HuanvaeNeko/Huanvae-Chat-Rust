use crate::storage::S3Client;
use tracing::info;

/// 头像服务
pub struct AvatarService;

impl AvatarService {
    /// 验证文件扩展名
    pub fn validate_extension(filename: &str) -> Result<String, String> {
        let allowed_extensions = ["jpg", "jpeg", "png", "gif", "webp"];
        
        let extension = filename
            .rsplit('.')
            .next()
            .ok_or("No file extension found")?
            .to_lowercase();

        if allowed_extensions.contains(&extension.as_str()) {
            Ok(extension)
        } else {
            Err(format!(
                "Unsupported file format. Allowed: {}",
                allowed_extensions.join(", ")
            ))
        }
    }

    /// 验证文件大小（最大 5MB）
    pub fn validate_size(data: &[u8]) -> Result<(), String> {
        const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB

        if data.len() > MAX_SIZE {
            Err(format!(
                "File too large. Maximum size: {} MB, got: {:.2} MB",
                MAX_SIZE / 1024 / 1024,
                data.len() as f64 / 1024.0 / 1024.0
            ))
        } else {
            Ok(())
        }
    }

    /// 上传头像
    pub async fn upload_avatar(
        s3_client: &S3Client,
        user_id: &str,
        data: Vec<u8>,
        filename: &str,
    ) -> Result<String, anyhow::Error> {
        // 验证文件大小
        Self::validate_size(&data)
            .map_err(|e| anyhow::anyhow!("File validation failed: {}", e))?;

        // 验证文件扩展名
        let extension = Self::validate_extension(filename)
            .map_err(|e| anyhow::anyhow!("File validation failed: {}", e))?;

        info!(
            "Uploading avatar for user: {}, size: {} bytes, extension: {}",
            user_id,
            data.len(),
            extension
        );

        // 上传到 MinIO
        let url = s3_client.upload_avatar(user_id, data, &extension).await?;

        info!("Avatar uploaded successfully: {}", url);
        Ok(url)
    }
}

