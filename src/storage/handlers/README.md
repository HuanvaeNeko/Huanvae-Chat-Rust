# Storage Handlers

HTTP 请求处理层，处理文件上传和访问相关的API请求。

## 文件说明

### `upload.rs`

处理文件上传相关接口：

- `request_upload()` - 请求上传（生成Token或预签名URL）
- `direct_upload()` - 直接上传文件（Token验证）
- `get_multipart_part_url()` - 获取分片上传URL

**好友文件上传**：当 `storage_location` 为 `friend_messages` 时，会自动验证好友关系。

### `file_access.rs`

处理个人文件访问接口：

- `generate_presigned_url()` - 生成普通文件预签名URL（3小时有效）
- `generate_extended_presigned_url()` - 生成超大文件预签名URL（自定义有效期）

### `friends_file_access.rs`

处理好友文件访问接口：

- `generate_friend_file_presigned_url()` - 生成好友文件预签名URL（3小时有效）
- `generate_friend_file_extended_presigned_url()` - 生成好友超大文件预签名URL

**访问验证流程**：
1. 查询 `file-uuid-mapping` 获取物理文件信息
2. 从 `physical_file_key` 解析 `conversation_uuid`
3. 验证请求用户是否是会话参与者
4. **实时验证当前好友关系**
5. 生成预签名URL

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
| `/file/{uuid}/presigned_url` | POST | `generate_presigned_url` | 是 | 个人文件预签名URL |
| `/file/{uuid}/presigned_url/extended` | POST | `generate_extended_presigned_url` | 是 | 个人文件扩展预签名URL |
| `/friends-file/{uuid}/presigned-url` | POST | `generate_friend_file_presigned_url` | 是 | 好友文件预签名URL |
| `/friends-file/{uuid}/presigned-url/extended` | POST | `generate_friend_file_extended_presigned_url` | 是 | 好友文件扩展预签名URL |
| `/files` | GET | `list_files_handler` | 是 | 文件列表查询 |

## 状态管理

```rust
pub struct StorageState {
    pub db: PgPool,
    pub file_service: Arc<FileService>,
    pub s3_client: Arc<S3Client>,
}
```

包含数据库连接池、文件服务和S3客户端的共享状态。

## 好友文件特殊处理

### 上传时
- 验证好友关系
- 文件存储到 `friends-file/{conversation-uuid}/{type}/` 路径
- 上传完成后自动授权双方访问权限

### 访问时
- 验证用户是会话参与者
- **实时验证好友关系**（删除好友后无法访问）
- 生成预签名URL

## 错误处理

所有handlers返回统一的错误格式：

```json
{
  "error": "错误描述"
}
```

HTTP状态码：
- 200: 成功
- 400: 参数错误（好友ID为空、非好友关系等）
- 401: Token无效
- 403: 无权访问（非会话参与者、好友关系已解除）
- 404: 文件不存在
- 500: 服务器错误







