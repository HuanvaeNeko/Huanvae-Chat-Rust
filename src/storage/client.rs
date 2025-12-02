use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::config::MinioConfig;

/// S3/MinIO 客户端封装
#[derive(Clone)]
pub struct S3Client {
    client: Arc<Client>,
    config: MinioConfig,
}

impl S3Client {
    /// 创建新的 S3 客户端
    pub async fn new(config: MinioConfig) -> Result<Self, anyhow::Error> {
        // 创建凭证
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "custom",
        );

        // 构建 S3 配置
        let s3_config = S3ConfigBuilder::new()
            .region(Region::new(config.region.clone()))
            .endpoint_url(&config.endpoint)
            .credentials_provider(credentials)
            .force_path_style(true) // MinIO 需要 path-style
            .behavior_version(BehaviorVersion::latest())
            .build();

        let client = Arc::new(Client::from_conf(s3_config));

        let s3_client = Self {
            client,
            config: config.clone(),
        };

        // 初始化 bucket
        s3_client.init_buckets().await?;

        Ok(s3_client)
    }

    /// 初始化所有必要的 buckets
    async fn init_buckets(&self) -> Result<(), anyhow::Error> {
        // 创建 avatars bucket（公开）
        self.create_bucket_if_not_exists(&self.config.bucket_avatars)
            .await?;
        
        // 设置头像 bucket 为公开读取
        self.set_bucket_public_read(&self.config.bucket_avatars)
            .await?;

        // 创建其他私有 buckets（按照 MinIO/data.md 规范）
        self.create_bucket_if_not_exists("user-file").await?;
        self.create_bucket_if_not_exists("friends-file").await?;
        self.create_bucket_if_not_exists("group-file").await?;

        Ok(())
    }

    /// 创建 bucket（如果不存在）
    async fn create_bucket_if_not_exists(&self, bucket: &str) -> Result<(), anyhow::Error> {
        match self.client.head_bucket().bucket(bucket).send().await {
            Ok(_) => {
                info!("Bucket '{}' already exists", bucket);
                Ok(())
            }
            Err(_) => {
                info!("Creating bucket '{}'", bucket);
                self.client
                    .create_bucket()
                    .bucket(bucket)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create bucket: {}", e))?;
                info!("Bucket '{}' created successfully", bucket);
                Ok(())
            }
        }
    }

    /// 设置 bucket 为公开读取
    async fn set_bucket_public_read(&self, bucket: &str) -> Result<(), anyhow::Error> {
        let policy = format!(
            r#"{{
                "Version": "2012-10-17",
                "Statement": [
                    {{
                        "Effect": "Allow",
                        "Principal": {{"AWS": ["*"]}},
                        "Action": ["s3:GetObject"],
                        "Resource": ["arn:aws:s3:::{}/*"]
                    }}
                ]
            }}"#,
            bucket
        );

        match self
            .client
            .put_bucket_policy()
            .bucket(bucket)
            .policy(policy)
            .send()
            .await
        {
            Ok(_) => {
                info!("Set bucket '{}' to public read", bucket);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to set bucket policy (may not be critical): {}", e);
                Ok(()) // 不阻止启动
            }
        }
    }

    /// 上传文件到指定 bucket
    pub async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, anyhow::Error> {
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload file: {}", e))?;

        // 返回公开访问 URL
        let url = format!("{}/{}/{}", self.config.public_url, bucket, key);
        Ok(url)
    }

    /// 上传头像
    pub async fn upload_avatar(
        &self,
        user_id: &str,
        data: Vec<u8>,
        extension: &str,
    ) -> Result<String, anyhow::Error> {
        let key = format!("{}.{}", user_id, extension);
        let content_type = match extension {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "application/octet-stream",
        };

        self.upload_file(&self.config.bucket_avatars, &key, data, content_type)
            .await
    }

    /// 删除文件
    pub async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), anyhow::Error> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))?;

        Ok(())
    }

    /// 读取文件内容
    pub async fn get_file(&self, bucket: &str, key: &str) -> Result<Vec<u8>, anyhow::Error> {
        let response = self.client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get file: {}", e))?;

        let data = response.body.collect().await
            .map_err(|e| anyhow::anyhow!("Failed to read file body: {}", e))?;

        Ok(data.to_vec())
    }

    /// 获取配置
    pub fn config(&self) -> &MinioConfig {
        &self.config
    }

    /// 生成预签名上传URL（PUT方法）
    pub async fn generate_presigned_upload_url(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        expires_in: u32,
    ) -> Result<String, anyhow::Error> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in as u64))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build presigning config: {}", e))?;

        let presigned = self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        Ok(presigned.uri().to_string())
    }

    /// 生成预签名下载URL（GET方法）
    pub async fn generate_presigned_download_url(
        &self,
        bucket: &str,
        key: &str,
        expires_in: u32,
    ) -> Result<String, anyhow::Error> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in as u64))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build presigning config: {}", e))?;

        let presigned = self.client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;

        Ok(presigned.uri().to_string())
    }

    /// 初始化分片上传（用于超大文件）
    pub async fn initiate_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
    ) -> Result<String, anyhow::Error> {
        let result = self.client
            .create_multipart_upload()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initiate multipart upload: {}", e))?;

        let upload_id = result.upload_id()
            .ok_or_else(|| anyhow::anyhow!("No upload_id returned"))?;

        Ok(upload_id.to_string())
    }

    /// 生成分片上传的预签名URL
    pub async fn generate_presigned_upload_part_url(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        expires_in: u32,
    ) -> Result<String, anyhow::Error> {
        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in as u64))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build presigning config: {}", e))?;

        let presigned = self.client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .presigned(presigning_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate presigned part URL: {}", e))?;

        Ok(presigned.uri().to_string())
    }

    /// 检查文件是否存在
    pub async fn file_exists(&self, bucket: &str, key: &str) -> Result<bool, anyhow::Error> {
        match self.client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 获取文件元数据
    pub async fn get_file_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<FileMetadata, anyhow::Error> {
        let result = self.client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get file metadata: {}", e))?;

        Ok(FileMetadata {
            content_length: result.content_length().unwrap_or(0) as u64,
            content_type: result.content_type().unwrap_or("application/octet-stream").to_string(),
            etag: result.e_tag().map(|s| s.to_string()),
            last_modified: result.last_modified().map(|dt| dt.secs()),
        })
    }
}

/// 文件元数据
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub content_length: u64,
    pub content_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<i64>,
}

