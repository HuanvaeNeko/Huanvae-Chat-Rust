# Storage Handlers

HTTP 请求处理层，处理文件上传相关的API请求。

## 文件说明

### `upload.rs`

处理文件上传相关接口：

- `request_upload()` - 请求上传（生成Token或预签名URL）
- `direct_upload()` - 直接上传文件（Token验证）
- `get_multipart_part_url()` - 获取分片上传URL

### `routes.rs`

定义storage模块的路由结构：

```rust
pub fn create_storage_routes(
    db: PgPool,
    s3_client: Arc<S3Client>,
    auth_state: AuthState,
    api_base_url: String,
) -> Router
```

返回配置好的Router，包含所有storage相关路由。

## 路由映射

| 路由 | 方法 | Handler | 鉴权 | 说明 |
|------|------|---------|------|------|
| `/upload/request` | POST | `request_upload` | 是 | 请求上传 |
| `/upload/direct` | POST | `direct_upload` | Token | 直接上传 |
| `/multipart/part-url` | GET | `get_multipart_part_url` | 是 | 分片URL |

## 状态管理

```rust
pub struct StorageState {
    pub file_service: Arc<FileService>,
    pub s3_client: Arc<S3Client>,
}
```

包含文件服务和S3客户端的共享状态。

## 错误处理

所有handlers返回统一的错误格式：

```json
{
  "error": "错误描述"
}
```

HTTP状态码：
- 200: 成功
- 400: 参数错误
- 401: Token无效
- 500: 服务器错误

