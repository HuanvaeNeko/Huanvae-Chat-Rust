use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
    primitives::ByteStream,
    Client,
};
use std::sync::Arc;
use tracing::{info, warn};

use super::config::S3Config;

/// S3/MinIO 客户端封装
#[derive(Clone)]
pub struct S3Client {
    client: Arc<Client>,
    config: S3Config,
}

impl S3Client {
    /// 创建新的 S3 客户端
    pub async fn new(config: S3Config) -> Result<Self, anyhow::Error> {
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
        self.create_bucket_if_not_exists(&self.config.bucket_avatars)
            .await?;
        
        // 设置头像 bucket 为公开读取
        self.set_bucket_public_read(&self.config.bucket_avatars)
            .await?;

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

    /// 获取配置
    pub fn config(&self) -> &S3Config {
        &self.config
    }
}

