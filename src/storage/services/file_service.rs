//! 统一文件服务（门面模式）
//!
//! FileService 作为文件操作的统一入口，内部委托给专门的子服务：
//! - FileUploadService: 文件上传相关操作
//! - FileDownloadService: 文件下载相关操作  
//! - FileQueryService: 文件查询相关操作
//!
//! 这种设计保持了对外API的稳定性，同时使内部代码更加模块化和可维护

use sqlx::PgPool;
use std::sync::Arc;

use crate::common::AppError;
use crate::storage::client::S3Client;
use crate::storage::models::*;
use crate::storage::services::{DeduplicationService, UuidMappingService};

use super::file_upload_service::FileUploadService;
use super::file_download_service::FileDownloadService;
use super::file_query_service::FileQueryService;

/// 统一文件服务（门面模式）
/// 
/// 提供统一的文件操作接口，内部委托给专门的子服务处理
pub struct FileService {
    // 子服务
    upload_service: FileUploadService,
    download_service: FileDownloadService,
    query_service: FileQueryService,
    // 共享组件（供需要直接访问的场景）
    s3_client: Arc<S3Client>,
    uuid_mapping_service: Arc<UuidMappingService>,
}

impl FileService {
    pub fn new(db: PgPool, s3_client: Arc<S3Client>, api_base_url: String) -> Self {
        let dedup_service = Arc::new(DeduplicationService::new(db.clone(), s3_client.clone()));
        let uuid_mapping_service = Arc::new(UuidMappingService::new(db.clone()));
        
        // 创建子服务
        let upload_service = FileUploadService::new(
            db.clone(),
            s3_client.clone(),
            dedup_service,
            uuid_mapping_service.clone(),
            api_base_url.clone(),
        );
        
        let download_service = FileDownloadService::new(
            db.clone(),
            s3_client.clone(),
        );
        
        let query_service = FileQueryService::new(
            db.clone(),
            api_base_url.clone(),
        );
        
        Self {
            upload_service,
            download_service,
            query_service,
            s3_client,
            uuid_mapping_service,
        }
    }

    // ========================================
    // 上传相关方法（委托给 FileUploadService）
    // ========================================

    /// 请求上传（统一入口）
    pub async fn request_upload(
        &self,
        user_id: &str,
        request: FileUploadRequest,
    ) -> Result<FileUploadResponse, AppError> {
        self.upload_service.request_upload(user_id, request).await
    }
    
    /// 验证并完成预签名上传（确认文件已上传到MinIO）
    pub async fn verify_and_complete_presigned_upload(
        &self,
        file_key: &str,
        user_id: &str,
    ) -> Result<FileRecord, AppError> {
        self.upload_service.verify_and_complete_presigned_upload(file_key, user_id).await
    }

    /// 完成预签名上传（创建UUID映射、权限授予）
    pub async fn complete_presigned_upload(
        &self,
        file_key: &str,
        owner_id: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &str,
        storage_location: &str,
        related_id: Option<&str>,
        file_hash: &str,
    ) -> Result<String, AppError> {
        self.upload_service.complete_presigned_upload(
            file_key, owner_id, file_size, content_type,
            preview_support, storage_location, related_id, file_hash
        ).await
    }

    /// 验证并获取上传Token信息
    pub async fn verify_and_get_upload_token(
        &self,
        token: &str,
    ) -> Result<FileRecord, AppError> {
        self.upload_service.verify_and_get_upload_token(token).await
    }

    /// 完成上传（消费Token）并创建UUID映射
    pub async fn complete_upload_with_token(
        &self,
        token: &str,
        actual_hash: &str,
        file_key: &str,
        owner_id: &str,
        file_size: i64,
        content_type: &str,
        preview_support: &str,
    ) -> Result<String, AppError> {
        self.upload_service.complete_upload_with_token(
            token, actual_hash, file_key, owner_id,
            file_size, content_type, preview_support
        ).await
    }

    /// 生成分片上传URL
    pub async fn generate_multipart_part_url(
        &self,
        file_key: &str,
        upload_id: &str,
        part_number: i32,
        user_id: &str,
    ) -> Result<MultipartPartResponse, AppError> {
        self.upload_service.generate_multipart_part_url(
            file_key, upload_id, part_number, user_id
        ).await
    }

    // ========================================
    // 下载相关方法（委托给 FileDownloadService）
    // ========================================

    /// 生成文件预签名下载URL（通过UUID访问）
    pub async fn generate_presigned_url(
        &self,
        user_id: &str,
        file_uuid: &str,
        expires_in: u32,
    ) -> Result<PresignedUrlResponse, AppError> {
        self.download_service.generate_presigned_url(user_id, file_uuid, expires_in).await
    }

    // ========================================
    // 查询相关方法（委托给 FileQueryService）
    // ========================================

    /// 查询用户文件列表（支持分页、过滤、排序）
    pub async fn list_user_files(
        &self,
        user_id: &str,
        page: i32,
        limit: i32,
        sort_by: String,
        sort_order: String,
    ) -> Result<FileListResponse, AppError> {
        self.query_service.list_user_files(user_id, page, limit, sort_by, sort_order).await
    }

    // ========================================
    // 工具方法（直接实现）
    // ========================================

    /// 获取bucket名称
    pub fn get_bucket_name(&self, storage_location: &StorageLocation) -> &str {
        storage_location.to_bucket_name()
    }

    /// 获取UUID映射信息
    pub async fn get_uuid_mapping(&self, file_uuid: &str) -> Result<Option<crate::storage::services::uuid_mapping::UuidMappingInfo>, AppError> {
        self.uuid_mapping_service.get_by_uuid(file_uuid).await
    }

    /// 生成文件URL
    pub fn generate_file_url(&self, storage_location: &StorageLocation, file_key: &str) -> String {
        let bucket = storage_location.to_bucket_name();
        format!("{}/{}/{}", self.s3_client.config().public_url, bucket, file_key)
    }
}

// ========================================
// 数据库模型结构
// ========================================

/// 数据库文件记录结构
#[derive(sqlx::FromRow)]
pub struct FileRecord {
    #[sqlx(rename = "file-key")]
    pub file_key: String,
    #[sqlx(rename = "owner-id")]
    pub owner_id: String,
    #[sqlx(rename = "file-type")]
    pub file_type: String,
    #[sqlx(rename = "storage-location")]
    pub storage_location: String,
    #[sqlx(rename = "related-id")]
    pub related_id: Option<String>,
    #[sqlx(rename = "file-size")]
    pub file_size: i64,
    #[sqlx(rename = "content-type")]
    pub content_type: String,
    #[sqlx(rename = "file-hash")]
    pub file_hash: String,
    #[sqlx(rename = "upload-token")]
    pub upload_token: Option<String>,
    #[sqlx(rename = "multipart-upload-id")]
    pub multipart_upload_id: Option<String>,
    pub status: String,
    #[sqlx(rename = "preview-support")]
    pub preview_support: String,
}

impl FileRecord {
    pub fn preview_support(&self) -> PreviewSupport {
        if self.preview_support == "inline_preview" {
            PreviewSupport::InlinePreview
        } else {
            PreviewSupport::DownloadOnly
        }
    }
}
