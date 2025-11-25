use std::env;

/// MinIO/S3 配置
#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket_avatars: String,
    pub public_url: String,
    pub region: String,
}

impl S3Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            endpoint: env::var("MINIO_ENDPOINT")
                .map_err(|_| "MINIO_ENDPOINT not set".to_string())?,
            access_key: env::var("MINIO_ACCESS_KEY")
                .map_err(|_| "MINIO_ACCESS_KEY not set".to_string())?,
            secret_key: env::var("MINIO_SECRET_KEY")
                .map_err(|_| "MINIO_SECRET_KEY not set".to_string())?,
            bucket_avatars: env::var("MINIO_BUCKET_AVATARS")
                .unwrap_or_else(|_| "avatars".to_string()),
            public_url: env::var("MINIO_PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            region: env::var("MINIO_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
        })
    }
}

