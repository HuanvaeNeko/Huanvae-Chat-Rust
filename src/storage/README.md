# Storage 模块

统一文件存储管理模块，提供文件上传、下载、预览等功能，支持头像、用户文件、好友消息文件、群聊文件等多种场景。

## 📂 目录结构

```
src/storage/
  ├─ client.rs         // S3/MinIO 客户端封装（支持预签名URL和分片上传）
  ├─ config.rs         // 配置管理（从环境变量加载）
  ├─ models/           // 数据模型层
  │   ├─ enums.rs      // 枚举类型定义（FileType, StorageLocation等）
  │   ├─ request.rs    // 请求结构体
  │   ├─ response.rs   // 响应结构体
  │   └─ mod.rs        // 模块导出
  ├─ services/         // 业务服务层
  │   ├─ avatar.rs     // 头像上传服务
  │   ├─ validator.rs  // 文件验证服务
  │   ├─ deduplication.rs  // 去重服务（哈希检测、秒传）
  │   ├─ file_service.rs   // 统一文件上传服务
  │   └─ mod.rs        // 服务模块导出
  ├─ handlers/         // HTTP 请求处理层
  │   ├─ upload.rs     // 上传接口
  │   ├─ routes.rs     // 路由定义
  │   └─ mod.rs        // 处理器模块导出
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

# API 基础URL（用于生成上传URL）
APP_BASE_URL=http://localhost:8080

# 文件上传配置（可选）
ENABLE_FILE_DEDUPLICATION=true        # 是否启用哈希去重
ENABLE_UPLOAD_RATE_LIMIT=false        # 是否启用并发限制
MAX_CONCURRENT_UPLOADS=5              # 最大并发上传数
```

## 🗂️ 存储结构（严格按照MinIO/data.md规范）

| Bucket | 用途 | 权限 | 路径规则 |
|--------|------|------|----------|
| `avatars` | 用户头像 | 公开读取 | `{user_id}.{ext}` |
| `user-file` | 用户个人文件 | 私有 | `{user_id}/{type}/{timestamp}_{hash}_{filename}` |
| `friends-file` | 好友消息文件 | 私有 | `{conversation_uuid}/{type}/{timestamp}_{hash}_{filename}` |
| `group-file` | 群聊文件 | 私有 | `{group_id}/{type}/{timestamp}_{hash}_{filename}` |

**type目录分类**：
- `images/` - 图片文件
- `videos/` - 视频文件
- `files/` - 文档文件

## 🚀 核心功能

### 1. 统一文件上传

#### **上传策略**

| 文件类型 | 大小范围 | 上传方式 | 预览支持 |
|---------|---------|---------|---------|
| 头像 | < 5MB | 一次性Token | 在线预览 |
| 图片 | < 100MB | 一次性Token | 在线预览 |
| 图片（大） | 100MB - 15GB | 一次性Token | 仅下载 |
| 视频 | < 15GB | 一次性Token | 在线预览 |
| 视频（大） | 15GB - 30GB | 一次性Token | 仅下载 |
| 文档 | < 15GB | 一次性Token | 仅下载 |
| 超大文件 | 15GB - 30GB | 预签名URL（分片） | 仅下载 |

#### **工作流程**

```
1. 客户端计算文件SHA-256哈希
2. 请求上传 → POST /api/storage/upload/request
   ├─ 秒传检查（哈希去重）
   ├─ 权限验证
   ├─ 文件类型和大小验证
   └─ 生成上传凭证（Token或预签名URL）
3. 客户端直连MinIO上传（使用Token或预签名URL）
4. 上传完成通知（Token自动验证和消费）
```

### 2. 秒传功能（哈希去重）

- 客户端计算文件SHA-256哈希
- 服务端检查数据库中是否已存在相同哈希的文件
- 如果存在且MinIO中文件可用，直接返回已有文件URL
- 支持`force_upload=true`强制重新上传

### 3. 一次性Token上传（< 15GB）

```javascript
// 前端示例
const uploadFile = async (file) => {
  // 1. 计算哈希
  const fileHash = await calculateSHA256(file);
  
  // 2. 请求上传
  const response = await fetch('/api/storage/upload/request', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      file_type: 'user_image',
      storage_location: 'user_files',
      filename: file.name,
      file_size: file.size,
      content_type: file.type,
      file_hash: fileHash,
      force_upload: false,
    }),
  });
  
  const { upload_url, instant_upload, existing_file_url } = await response.json();
  
  // 3. 检查秒传
  if (instant_upload) {
    console.log('秒传成功！', existing_file_url);
    return existing_file_url;
  }
  
  // 4. 上传文件
  const formData = new FormData();
  formData.append('file', file);
  
  const uploadResponse = await fetch(upload_url, {
    method: 'POST',
    body: formData,
  });
  
  return await uploadResponse.json();
};
```

### 4. 预签名URL分片上传（> 15GB）

```javascript
// 超大文件分片上传示例
const uploadLargeFile = async (file) => {
  // 1. 请求上传（获取multipart_upload_id）
  const { multipart_upload_id, file_key } = await requestUpload(file);
  
  // 2. 分片上传
  const chunkSize = 50 * 1024 * 1024; // 50MB
  const chunks = Math.ceil(file.size / chunkSize);
  
  for (let i = 0; i < chunks; i++) {
    const start = i * chunkSize;
    const end = Math.min(start + chunkSize, file.size);
    const chunk = file.slice(start, end);
    
    // 获取分片URL
    const { part_url } = await fetch(
      `/api/storage/multipart/part-url?file_key=${file_key}&upload_id=${multipart_upload_id}&part_number=${i + 1}`,
      { headers: { 'Authorization': `Bearer ${accessToken}` } }
    ).then(r => r.json());
    
    // 上传分片
    await fetch(part_url, {
      method: 'PUT',
      body: chunk,
    });
  }
};
```

## 📡 API 接口

### POST /api/storage/upload/request

请求文件上传（需鉴权）

**Request Body:**
```json
{
  "file_type": "user_image",
  "storage_location": "user_files",
  "related_id": "friend_user_id",
  "filename": "example.jpg",
  "file_size": 1048576,
  "content_type": "image/jpeg",
  "file_hash": "abc123...",
  "estimated_upload_time": 3600,
  "force_upload": false
}
```

**Response:**
```json
{
  "mode": "one_time_token",
  "preview_support": "inline_preview",
  "upload_token": "token_xxx",
  "upload_url": "http://localhost:8080/api/storage/upload/direct?token=xxx",
  "expires_in": 900,
  "file_key": "user_id/images/timestamp_hash_filename.jpg",
  "max_file_size": 104857600,
  "instant_upload": false,
  "existing_file_url": null
}
```

### POST /api/storage/upload/direct?token={token}

直接上传文件（Token验证，无需access_token）

**Request:** multipart/form-data
- `file`: 文件数据

**Response:**
```json
{
  "file_url": "http://localhost:9000/user-file/...",
  "file_key": "user_id/images/...",
  "file_size": 1048576,
  "content_type": "image/jpeg",
  "preview_support": "inline_preview"
}
```

### GET /api/storage/multipart/part-url

获取分片上传URL（需鉴权）

**Query Parameters:**
- `file_key`: 文件key
- `upload_id`: 分片上传ID
- `part_number`: 分片编号

**Response:**
```json
{
  "part_url": "http://localhost:9000/...",
  "part_number": 1,
  "expires_in": 3600
}
```

## 🔒 安全特性

1. **哈希验证**：服务端验证上传后的文件哈希与客户端提供的哈希是否匹配
2. **大小验证**：验证实际上传的文件大小与声明的大小是否一致
3. **一次性Token**：每个Token只能使用一次，使用后自动失效
4. **权限验证**：好友文件需验证好友关系，群文件需验证群成员关系
5. **时效控制**：
   - 小文件（< 10MB）：5分钟
   - 中等文件（10-100MB）：15分钟
   - 大文件（1-10GB）：2小时
   - 超大文件（> 10GB）：用户指定（最多7天）
6. **自动清理**：定时清理过期的pending记录

## 📊 数据库设计

### file_records 表

存储所有文件的元数据记录。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| file_key | VARCHAR(500) | 文件key（唯一） |
| file_url | TEXT | 访问URL |
| file_hash | VARCHAR(64) | SHA-256哈希 |
| owner_id | VARCHAR(255) | 所有者ID |
| file_size | BIGINT | 文件大小 |
| status | VARCHAR(20) | 状态（pending/completed/failed） |
| upload_token | VARCHAR(128) | 一次性上传Token |
| preview_support | VARCHAR(20) | 预览支持 |
| expires_at | TIMESTAMPTZ | 过期时间 |

### user_storage_quotas 表

用户存储配额管理。

| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | VARCHAR(255) | 用户ID |
| total_quota | BIGINT | 总配额（默认10GB） |
| used_space | BIGINT | 已使用空间 |
| file_count | INTEGER | 文件数量 |

## 🎯 设计原则

- **直连上传**：大文件直连MinIO，不占用后端带宽
- **安全可控**：多重验证确保文件安全
- **高性能**：秒传、去重节省带宽和存储
- **易扩展**：模块化设计，易于添加新功能
- **类型安全**：充分利用Rust类型系统

## 📚 相关文档

- [MinIO数据结构规范](../../MinIO/data.md)
- [数据库迁移](../../PostgreSQL/init/05_file_records.sql)

