use serde::{Deserialize, Serialize};

use super::enums::{PreviewSupport, UploadMode};

/// 上传响应
#[derive(Debug, Serialize)]
pub struct FileUploadResponse {
    pub mode: UploadMode,
    pub preview_support: PreviewSupport,
    
    // 一次性Token模式字段
    pub upload_token: Option<String>,
    pub upload_url: Option<String>,
    pub expires_in: Option<u32>,
    
    // 预签名URL模式字段（超大文件）
    pub presigned_url: Option<String>,
    pub multipart_upload_id: Option<String>,
    
    pub file_key: String,
    pub max_file_size: u64,
    
    /// 秒传标识（文件已存在）
    pub instant_upload: bool,
    pub existing_file_url: Option<String>,
}

/// 文件上传完成响应
#[derive(Debug, Serialize)]
pub struct FileCompleteResponse {
    pub file_url: String,
    pub file_key: String,
    pub file_size: u64,
    pub content_type: String,
    pub preview_support: PreviewSupport,
}

/// 下载响应
#[derive(Debug, Serialize)]
pub struct FileDownloadResponse {
    pub download_url: String,
    pub expires_in: i64,
}

/// 分片上传分片URL响应
#[derive(Debug, Serialize)]
pub struct MultipartPartResponse {
    pub part_url: String,
    pub part_number: i32,
    pub expires_in: u32,
}

/// 已存在文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingFileInfo {
    pub file_key: String,
    pub file_url: String,
    pub file_size: i64,
    pub content_type: String,
}

/// 预签名URL响应
#[derive(Debug, Serialize)]
pub struct PresignedUrlResponse {
    /// 预签名URL（直连MinIO）
    pub presigned_url: String,
    
    /// 过期时间（ISO 8601格式）
    pub expires_at: String,
    
    /// 文件UUID
    pub file_uuid: String,
    
    /// 文件大小
    pub file_size: i64,
    
    /// 文件类型
    pub content_type: String,
    
    /// 警告信息（可选）
    pub warning: Option<String>,
}

