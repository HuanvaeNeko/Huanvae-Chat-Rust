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
1. 客户端计算文件采样SHA-256哈希
   - 小文件（< 30MB）：完整哈希
   - 大文件（≥ 30MB）：采样哈希（元信息 + 开头/中间/结尾各10MB）
2. 请求上传 → POST /api/storage/upload/request
   ├─ 秒传检查（采样哈希去重）
   ├─ 权限验证
   ├─ 文件类型和大小验证
   └─ 生成上传凭证（Token或预签名URL）
3. 客户端直连MinIO上传（使用Token或预签名URL）
4. 上传完成通知（Token自动验证和消费）
```

### 2. 秒传功能（UUID映射 + 权限表 + 采样哈希）

**核心机制**：
- 客户端计算文件采样SHA-256哈希（小文件完整哈希，大文件采样哈希）
- 服务端查询`file_uuid_mapping`表，检查是否存在相同哈希
- **首次上传**：
  - 上传文件到用户专属目录
  - 生成唯一UUID作为访问标识
  - 创建UUID映射记录
  - 授予上传者`owner`权限
- **秒传（不同用户上传相同文件）**：
  - 查询到已存在的UUID映射（采样哈希匹配）
  - 为当前用户创建独立的file_records记录
  - 授予当前用户访问权限
  - 返回相同的UUID访问URL
  - **无需重复上传物理文件**

**采样哈希说明**：
- 小文件（< 30MB）：计算完整SHA-256哈希
- 大文件（≥ 30MB）：采样策略
  - 文件元信息（文件名 + 大小 + 修改时间 + MIME类型）
  - 开头10MB数据
  - 中间10MB数据
  - 结尾10MB数据
  - 合并计算SHA-256哈希
- **优势**：避免浏览器内存溢出，3GB视频也能快速计算哈希并实现秒传
- **唯一性**：元信息+三个采样点足以唯一识别文件，误判概率极低

**UUID映射机制优势**：
- ✅ **真正的跨用户去重**：相同文件只存储一份物理副本
- ✅ **统一的访问控制**：基于UUID和权限表，不依赖MinIO策略
- ✅ **灵活的权限管理**：支持软删除（revoked_at）
- ✅ **用户隔离**：每个用户有独立的file_key，但共享物理文件
- ✅ **秒传体验**：仅需数据库操作即可完成（< 10ms）

**访问URL格式**：
```
旧格式（不推荐）: http://localhost:9000/user-file/{user_id}/images/xxx.jpg
新格式（UUID）  : http://localhost:8080/api/storage/file/{uuid}
```

### 3. 一次性Token上传（< 15GB）

```javascript
// 前端示例
const uploadFile = async (file) => {
  // 1. 计算采样哈希
  const fileHash = await calculateSamplingHashSHA256(file);
  
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

**说明**：
- `file_hash`: 采样SHA-256哈希（小文件完整哈希，大文件采样哈希）
- `estimated_upload_time`: 仅超大文件（> 15GB）需要提供

**Response (首次上传):**
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

**Response (秒传):**
```json
{
  "mode": "one_time_token",
  "preview_support": "inline_preview",
  "upload_token": null,
  "upload_url": null,
  "expires_in": null,
  "file_key": "user_id/images/timestamp_hash_filename.jpg",
  "max_file_size": 0,
  "instant_upload": true,
  "existing_file_url": "http://localhost:8080/api/storage/file/{uuid}"
}
```

### POST /api/storage/upload/direct?token={token}

直接上传文件（Token验证，无需access_token）

**Request:** multipart/form-data
- `file`: 文件数据

**Response:**
```json
{
  "file_url": "http://localhost:8080/api/storage/file/{uuid}",
  "file_key": "user_id/images/...",
  "file_size": 1048576,
  "content_type": "image/jpeg",
  "preview_support": "inline_preview"
}
```

### POST /api/storage/file/{uuid}/presigned-url

生成文件预签名下载URL（普通文件，3小时有效）

**Headers:**
- `Authorization: Bearer {access_token}`

**Request Body:**
```json
{
  "operation": "download",
  "expires_in": 10800  // 可选，默认3小时（10800秒），最大3小时
}
```

**Response:**
```json
{
  "presigned_url": "http://localhost:9000/user-file/xxx?X-Amz-Signature=...",
  "expires_at": "2025-11-30T23:59:59Z",
  "file_uuid": "d2f612d5-70b0-4d4e-8779-86cf6aeb2b30",
  "file_size": 1048576,
  "content_type": "image/jpeg"
}
```

**说明**：
- 通过UUID获取文件的预签名下载URL
- 验证用户权限（file_access_permissions表）
- 普通文件固定3小时有效期
- 前端应缓存预签名URL，避免重复请求

### POST /api/storage/file/{uuid}/presigned-url/extended

生成文件预签名下载URL（超大文件，自定义有效期）

**Headers:**
- `Authorization: Bearer {access_token}`

**Request Body:**
```json
{
  "operation": "download",
  "estimated_download_time": 86400  // 必填，预计下载时间（秒），如24小时
}
```

**Response:**
```json
{
  "presigned_url": "http://localhost:9000/user-file/xxx?X-Amz-Signature=...",
  "expires_at": "2025-12-01T20:00:00Z",
  "file_uuid": "...",
  "file_size": 30000000000,
  "content_type": "video/mp4",
  "warning": "此链接将在24小时后过期"
}
```

**说明**：
- 用于超大文件（> 15GB）
- 有效期：最少3小时（10800秒），最多7天（604800秒）
- 前端需提示用户输入预计下载时间

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

1. **采样哈希验证**：
   - 小文件：完整SHA-256哈希验证
   - 大文件：采样哈希用于去重检查，不做完整性验证
   - 服务端跳过哈希验证以避免内存溢出
2. **大小验证**：验证实际上传的文件大小与声明的大小是否一致
3. **一次性Token**：每个Token只能使用一次，使用后自动失效
4. **权限验证**：好友文件需验证好友关系，群文件需验证群成员关系
5. **预签名URL安全**：
   - 普通文件：3小时有效期
   - 超大文件（> 15GB）：自定义有效期（3小时-7天）
   - URL过期前5分钟前端自动提醒刷新
   - 实时权限验证：生成URL前检查file_access_permissions表
   - 时效性权限：URL在有效期内可用，无法中途撤销
6. **时效控制**：
   - 小文件（< 10MB）：5分钟
   - 中等文件（10-100MB）：15分钟
   - 大文件（1-10GB）：2小时
   - 超大文件（> 10GB）：用户指定（最多7天）
7. **自动清理**：定时清理过期的pending记录

## 📊 数据库设计

### file_records 表

存储所有文件的元数据记录。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| file_key | VARCHAR(500) | 文件key（唯一，用户的虚拟路径） |
| file_url | TEXT | 访问URL（UUID格式） |
| file_hash | VARCHAR(64) | SHA-256哈希 |
| **file_uuid** | **VARCHAR(36)** | **关联UUID映射表** |
| physical_file_key | VARCHAR(500) | 物理文件key（已废弃，被UUID机制取代） |
| owner_id | VARCHAR(255) | 所有者ID |
| file_size | BIGINT | 文件大小 |
| status | VARCHAR(20) | 状态（pending/completed/failed） |
| upload_token | VARCHAR(128) | 一次性上传Token |
| preview_support | VARCHAR(20) | 预览支持 |
| expires_at | TIMESTAMPTZ | 过期时间 |

### file_uuid_mapping 表

UUID映射表，实现跨用户去重。

| 字段 | 类型 | 说明 |
|------|------|------|
| uuid | VARCHAR(36) | 主键，随机UUID |
| physical_file_key | VARCHAR(500) | 实际物理文件路径 |
| file_hash | VARCHAR(64) | SHA-256哈希（用于查重） |
| file_size | BIGINT | 文件大小 |
| content_type | VARCHAR(100) | MIME类型 |
| preview_support | VARCHAR(20) | 预览支持 |
| first_uploader_id | VARCHAR(255) | 首次上传者（审计用） |
| created_at | TIMESTAMPTZ | 创建时间 |

### file_access_permissions 表

文件访问权限表，控制谁可以访问。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| file_uuid | VARCHAR(36) | 关联映射表的UUID |
| user_id | VARCHAR(255) | 有权访问的用户ID |
| access_type | VARCHAR(20) | 权限类型（owner/read） |
| granted_by | VARCHAR(50) | 授权来源（upload/share/friend/group） |
| related_context | VARCHAR(500) | 关联上下文（好友ID/群ID等） |
| granted_at | TIMESTAMPTZ | 授权时间 |
| revoked_at | TIMESTAMPTZ | 软删除时间 |

### file_references 表

文件引用表（已废弃，被UUID机制取代）。

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

