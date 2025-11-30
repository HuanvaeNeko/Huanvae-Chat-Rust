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

/// 预签名URL请求（通过UUID访问）
#[derive(Debug, Deserialize)]
pub struct PresignedUrlRequest {
    /// 操作类型：download 或 preview
    pub operation: Option<String>,
    
    /// 有效期（秒），普通文件最长3小时
    pub expires_in: Option<u32>,
    
    /// 预计下载时间（秒），仅超大文件需要
    pub estimated_download_time: Option<u32>,
}

/// 文件列表查询请求
#[derive(Debug, Deserialize)]
pub struct FileListQuery {
    /// 文件类型过滤（可选）
    pub file_type: Option<String>,
    
    /// 存储位置过滤（可选）
    pub storage_location: Option<String>,
    
    /// 页码（从1开始，默认1）
    pub page: Option<i32>,
    
    /// 每页数量（默认20，最大100）
    pub limit: Option<i32>,
    
    /// 排序字段（created_at, file_size）
    pub sort_by: Option<String>,
    
    /// 排序方向（asc, desc）
    pub sort_order: Option<String>,
}

