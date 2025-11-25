# Storage 模块

对象存储（MinIO/S3）客户端封装模块，提供文件上传、下载、删除等功能，支持头像、群文件、用户文件等多种场景。

## 📂 目录结构

```
src/storage/
  ├─ client.rs         // S3/MinIO 客户端封装
  ├─ config.rs         // 配置管理（从环境变量加载）
  ├─ services/         // 业务服务层
  │   ├─ avatar.rs     // 头像上传服务
  │   └─ mod.rs        // 服务模块导出
  └─ mod.rs            // 模块导出
```

## 🔧 配置（环境变量）

在 `.env` 文件中配置以下参数：

```env
# MinIO 对象存储配置
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin123
MINIO_BUCKET_AVATARS=avatars
MINIO_PUBLIC_URL=http://localhost:9000
MINIO_REGION=us-east-1
```

## 🗂️ Bucket 分区

根据 `MinIO/data.md` 的规划：

| Bucket | 用途 | 权限 | 路径规则 |
|--------|------|------|----------|
| `avatars` | 用户头像 | 公开读取 | `{user_id}.{ext}` |
| `group-files` | 群聊文件 | 私有 | `{group_id}/{type}/{filename}` |
| `user-files` | 用户文件 | 私有 | `{user_id}/{type}/{filename}` |

**当前实现**：
- ✅ `avatars` bucket（已自动创建并设置为公开读取）
- ⏳ `group-files`（待实现）
- ⏳ `user-files`（待实现）

## 🏗️ 核心组件

### S3Client (`client.rs`)

S3/MinIO 客户端封装，提供以下功能：

```rust
impl S3Client {
    /// 创建新的 S3 客户端并初始化 buckets
    pub async fn new(config: S3Config) -> Result<Self, anyhow::Error>
    
    /// 上传文件到指定 bucket
    pub async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, anyhow::Error>
    
    /// 上传头像（便捷方法）
    pub async fn upload_avatar(
        &self,
        user_id: &str,
        data: Vec<u8>,
        extension: &str,
    ) -> Result<String, anyhow::Error>
    
    /// 删除文件
    pub async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), anyhow::Error>
}
```

### S3Config (`config.rs`)

配置结构体，从环境变量加载：

```rust
pub struct S3Config {
    pub endpoint: String,           // MinIO 端点
    pub access_key: String,         // 访问密钥
    pub secret_key: String,         // 密钥
    pub bucket_avatars: String,     // 头像 bucket 名称
    pub public_url: String,         // 公开访问 URL
    pub region: String,             // 区域
}

impl S3Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, String>
}
```

### AvatarService (`services/avatar.rs`)

头像上传业务逻辑：

```rust
impl AvatarService {
    /// 验证文件扩展名（仅允许：jpg, jpeg, png, gif, webp）
    pub fn validate_extension(filename: &str) -> Result<String, String>
    
    /// 验证文件大小（最大 5MB）
    pub fn validate_size(data: &[u8]) -> Result<(), String>
    
    /// 上传头像（包含验证）
    pub async fn upload_avatar(
        s3_client: &S3Client,
        user_id: &str,
        data: Vec<u8>,
        filename: &str,
    ) -> Result<String, anyhow::Error>
}
```

## 🔄 使用示例

### 初始化客户端

```rust
use huanvae_chat::storage::{S3Client, S3Config};

// 在 main.rs 中初始化
let s3_config = S3Config::from_env()
    .expect("Failed to load MinIO configuration");
let s3_client = Arc::new(
    S3Client::new(s3_config)
        .await
        .expect("Failed to initialize S3 client")
);
```

### 上传头像

```rust
use huanvae_chat::storage::services::AvatarService;

// 在 handler 中使用
let avatar_url = AvatarService::upload_avatar(
    &s3_client,
    "user-123",
    file_data,
    "avatar.jpg"
).await?;

// 返回的 URL 格式：
// http://localhost:9000/avatars/user-123.jpg
```

### 直接上传文件

```rust
let url = s3_client.upload_file(
    "avatars",                    // bucket
    "user-123.jpg",               // key
    file_data,                    // data
    "image/jpeg"                  // content-type
).await?;
```

## 🔒 安全特性

1. **文件类型验证**
   - 头像仅允许：jpg, jpeg, png, gif, webp
   - 通过文件扩展名验证

2. **文件大小限制**
   - 头像最大 5MB
   - 可根据业务需求调整

3. **权限隔离**
   - 头像 bucket 公开读取（便于直接访问）
   - 其他 buckets 私有访问（需签名 URL）

4. **路径规范**
   - 头像：`{user_id}.{ext}`
   - 防止路径遍历攻击

## 📝 初始化行为

在应用启动时，`S3Client::new()` 会自动执行：

1. ✅ 创建 `avatars` bucket（如果不存在）
2. ✅ 设置 `avatars` bucket 为公开读取
3. ✅ 验证连接是否正常

如果初始化失败，应用将无法启动并输出错误日志。

## 🚀 扩展指南

### 添加新的文件类型支持

1. 在 `services/` 中创建新的服务文件（如 `group_file.rs`）
2. 实现验证和上传逻辑
3. 在 `client.rs` 中添加便捷方法（可选）
4. 更新 `services/mod.rs` 导出

### 添加新的 Bucket

1. 在 `S3Config` 中添加 bucket 名称字段
2. 在 `S3Client::init_buckets()` 中创建 bucket
3. 根据需求设置 bucket 权限策略

## 🔗 依赖

```toml
aws-sdk-s3 = "1.115.0"
aws-config = "1.8.11"
aws-credential-types = "1.2.10"
```

使用 AWS SDK for Rust 来与 MinIO 交互（MinIO 完全兼容 S3 API）。

## 🎯 设计原则

- **简单易用**：提供高层次的便捷方法
- **类型安全**：利用 Rust 类型系统保证正确性
- **错误处理**：使用 `anyhow::Error` 统一错误类型
- **异步优先**：所有 IO 操作使用 `async/await`
- **可测试性**：业务逻辑与存储层分离

