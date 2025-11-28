# Storage Models

数据模型层，定义文件存储相关的数据结构。

## 文件说明

### `enums.rs`

定义核心枚举类型：

#### FileType（文件类型）
```rust
pub enum FileType {
    Avatar,              // 头像（< 5MB）
    UserImage,           // 用户图片（< 100MB）
    UserImageFile,       // 图片文件模式（100MB - 15GB）
    UserVideo,           // 用户视频（< 15GB）
    UserVideoFile,       // 视频文件模式（15GB - 30GB）
    UserDocument,        // 用户文档（< 15GB）
    FriendImage,         // 好友聊天图片
    FriendImageFile,     // 好友聊天图片文件模式
    FriendVideo,         // 好友聊天视频
    FriendVideoFile,     // 好友聊天视频文件模式
    FriendDocument,      // 好友聊天文档
    GroupImage,          // 群聊图片
    GroupVideo,          // 群聊视频
    GroupDocument,       // 群聊文档
}
```

#### StorageLocation（存储位置）
```rust
pub enum StorageLocation {
    Avatars,          // avatars bucket
    UserFiles,        // user-file bucket
    FriendMessages,   // friends-file bucket
    GroupFiles,       // group-file bucket
}
```

#### UploadMode（上传模式）
```rust
pub enum UploadMode {
    OneTimeToken,     // 一次性Token（< 15GB）
    PresignedUrl,     // 预签名URL（> 15GB）
}
```

#### PreviewSupport（预览支持）
```rust
pub enum PreviewSupport {
    InlinePreview,    // 支持在线预览
    DownloadOnly,     // 仅支持下载
}
```

### `request.rs`

定义请求结构体：

#### FileUploadRequest
文件上传请求：
```rust
pub struct FileUploadRequest {
    pub file_type: FileType,
    pub storage_location: StorageLocation,
    pub related_id: Option<String>,
    pub filename: String,
    pub file_size: u64,
    pub content_type: String,
    pub file_hash: String,                 // SHA-256哈希
    pub force_upload: Option<bool>,
    pub estimated_upload_time: Option<u32>,
}
```

#### UploadCompleteRequest
上传完成通知：
```rust
pub struct UploadCompleteRequest {
    pub file_key: String,
    pub storage_location: StorageLocation,
    pub file_size: u64,
    pub etag: Option<String>,
}
```

### `response.rs`

定义响应结构体：

#### FileUploadResponse
上传响应（包含上传凭证）：
```rust
pub struct FileUploadResponse {
    pub mode: UploadMode,
    pub preview_support: PreviewSupport,
    pub upload_token: Option<String>,
    pub upload_url: Option<String>,
    pub expires_in: Option<u32>,
    pub file_key: String,
    pub max_file_size: u64,
    pub instant_upload: bool,              // 秒传标识
    pub existing_file_url: Option<String>,
    // ... 预签名URL字段
}
```

#### FileCompleteResponse
文件上传完成响应：
```rust
pub struct FileCompleteResponse {
    pub file_url: String,
    pub file_key: String,
    pub file_size: u64,
    pub content_type: String,
    pub preview_support: PreviewSupport,
}
```

## 数据验证

使用 `validator` crate 进行数据验证：

```rust
#[derive(Validate)]
pub struct FileUploadRequest {
    #[validate(length(equal = 64))]
    pub file_hash: String,  // 必须是64位十六进制
    
    #[validate(range(min = 3600, max = 604800))]
    pub estimated_upload_time: Option<u32>,  // 1小时到7天
}
```

## 序列化

所有结构体都实现了 `Serialize` 和 `Deserialize`，使用 serde 进行JSON序列化。

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Example { ... }
```

