# Storage Services

对象存储业务服务层，提供具体文件类型的上传、验证和管理功能。

## 📁 文件列表

- `avatar.rs` - 头像上传服务
- `file_service.rs` - 统一文件上传服务
- `deduplication.rs` - 去重/秒传服务
- `uuid_mapping.rs` - UUID映射服务
- `validator.rs` - 文件验证服务
- `mod.rs` - 模块导出

## 🚀 DeduplicationService（秒传核心）

### 功能

检查文件哈希是否已存在，实现秒传功能。

### 核心方法

#### `check_and_create_uuid_reference()`

```rust
pub async fn check_and_create_uuid_reference(
    &self,
    file_hash: &str,
    user_id: &str,
    file_type: &FileType,
    storage_location: &StorageLocation,
    related_id: Option<&str>,
    new_file_key: &str,
    file_size: i64,
    content_type: &str,
    preview_support: &PreviewSupport,
) -> Result<Option<ExistingFileInfo>, AppError>
```

**流程**：
1. 查询 `file-uuid-mapping` 表是否存在相同哈希
2. 验证 MinIO 中物理文件是否存在
3. 授予当前用户访问权限
4. **群文件秒传**：自动授予所有活跃群成员读取权限
5. **好友文件秒传**：自动授予好友读取权限
6. 创建 `file-records` 记录
7. 返回已存在文件信息（秒传成功）或 `None`（需要实际上传）

## 🖼️ AvatarService

### 功能

提供头像文件的验证、上传和管理功能。

### 支持的文件格式

- `jpg` / `jpeg` - JPEG 图片
- `png` - PNG 图片
- `gif` - GIF 动图
- `webp` - WebP 图片

### 文件大小限制

- 最大：10 MB (10,485,760 bytes)

### 存储路径

```
avatars/{user_id}.{extension}
```

**示例**：
- `avatars/user-123.jpg`
- `avatars/testuser001.png`

### 公开方法

#### `validate_extension()`

验证文件扩展名是否合法。

```rust
pub fn validate_extension(filename: &str) -> Result<String, String>
```

**参数**：
- `filename`: 文件名（如 "avatar.jpg"）

**返回**：
- `Ok(extension)`: 验证通过，返回小写扩展名
- `Err(message)`: 验证失败，返回错误信息

**示例**：
```rust
let ext = AvatarService::validate_extension("user_avatar.jpg")?;
// ext = "jpg"

// 不支持的格式会返回错误
AvatarService::validate_extension("file.pdf")?;
// Err("Unsupported file format. Allowed: jpg, jpeg, png, gif, webp")
```

#### `validate_size()`

验证文件大小是否在限制范围内。

```rust
pub fn validate_size(data: &[u8]) -> Result<(), String>
```

**参数**：
- `data`: 文件字节数据

**返回**：
- `Ok(())`: 验证通过
- `Err(message)`: 文件过大，返回错误信息

**示例**：
```rust
AvatarService::validate_size(&file_data)?;
// 如果文件大于 5MB，返回类似：
// "File too large. Maximum size: 5 MB, got: 8.42 MB"
```

#### `upload_avatar()`

上传头像到 MinIO（包含完整验证）。

```rust
pub async fn upload_avatar(
    s3_client: &S3Client,
    user_id: &str,
    data: Vec<u8>,
    filename: &str,
) -> Result<String, anyhow::Error>
```

**参数**：
- `s3_client`: S3 客户端引用
- `user_id`: 用户 ID
- `data`: 文件数据
- `filename`: 原始文件名（用于提取扩展名）

**返回**：
- `Ok(url)`: 上传成功，返回公开访问 URL
- `Err(e)`: 上传失败或验证失败

**处理流程**：
1. 验证文件大小（≤ 10MB）
2. 验证文件扩展名（jpg/png/gif/webp）
3. 上传到 MinIO `avatars` bucket
4. 返回公开访问 URL

**示例**：
```rust
use huanvae_chat::storage::{S3Client, services::AvatarService};

let url = AvatarService::upload_avatar(
    &s3_client,
    "user-123",
    file_bytes,
    "my_photo.jpg"
).await?;

println!("Avatar URL: {}", url);
// 输出: Avatar URL: http://localhost:9000/avatars/user-123.jpg
```

## 🔮 未来扩展

### GroupFileService（已集成到 FileService）

群聊文件上传已集成到统一的 `FileService`，通过 `storage_location: group_files` 区分。

**存储路径**：
```
group-file/group-{group_id}/files/{timestamp}_{hash}_{filename}
group-file/group-{group_id}/videos/{timestamp}_{hash}_{filename}
group-file/group-{group_id}/images/{timestamp}_{hash}_{filename}
```

**权限机制**：
- 上传者自动获得 `owner` 权限
- 所有活跃群成员自动获得 `read` 权限
- 秒传时同样会授予群成员权限

### UserFileService（待实现）

用户个人文件上传服务。

**存储路径**：
```
user-files/{user_id}/files/{filename}
user-files/{user_id}/videos/{filename}
user-files/{user_id}/images/{filename}
```

## 📋 实现新服务的步骤

1. **创建新文件**：在 `services/` 下创建（如 `group_file.rs`）

2. **定义服务结构**：
```rust
pub struct GroupFileService;

impl GroupFileService {
    // 实现验证和上传方法
}
```

3. **更新 mod.rs**：
```rust
pub mod avatar;
pub mod group_file;  // 新增

pub use avatar::AvatarService;
pub use group_file::GroupFileService;  // 新增
```

4. **编写测试**：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_extension() {
        // 测试代码
    }
}
```

## 🎯 设计原则

- **单一职责**：每个服务处理一种文件类型
- **验证优先**：先验证后上传，减少无效请求
- **错误友好**：提供清晰的错误信息
- **可扩展性**：便于添加新的文件类型支持

