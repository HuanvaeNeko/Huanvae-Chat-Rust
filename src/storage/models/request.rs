use serde::Deserialize;
use validator::Validate;

use super::enums::{FileType, StorageLocation};

/// 文件上传请求（统一入口）
#[derive(Debug, Deserialize, Validate)]
pub struct FileUploadRequest {
    pub file_type: FileType,
    pub storage_location: StorageLocation,
    pub related_id: Option<String>,  // 好友ID或群ID
    pub filename: String,
    
    pub file_size: u64,
    
    pub content_type: String,
    
    /// 文件哈希（SHA-256，客户端计算，采样哈希）
    /// 采样策略：文件元信息 + 开头10MB + 中间10MB + 结尾10MB
    pub file_hash: Option<String>,
    
    /// 预计上传时间（秒，仅超大文件需要）
    #[validate(range(min = 3600, max = 604800))] // 1小时到7天
    pub estimated_upload_time: Option<u32>,
    
    /// 是否强制上传（即使哈希重复）
    pub force_upload: Option<bool>,
}

/// 上传完成通知
#[derive(Debug, Deserialize)]
pub struct UploadCompleteRequest {
    pub file_key: String,
    pub storage_location: StorageLocation,
    pub file_size: u64,
    pub etag: Option<String>,
}

/// 预签名下载URL请求
#[derive(Debug, Deserialize, Validate)]
pub struct PresignedDownloadRequest {
    pub file_key: String,
    pub storage_location: StorageLocation,
    #[validate(range(min = 300, max = 3600))]
    pub expires_in: Option<u32>,
}

/// 分片上传分片URL请求
#[derive(Debug, Deserialize)]
pub struct MultipartPartRequest {
    pub file_key: String,
    pub upload_id: String,
    pub part_number: i32,
}

