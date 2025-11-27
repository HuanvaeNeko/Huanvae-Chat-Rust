# MinIO 对象存储结构说明

## 📁 目录结构

MinIO 对象存储的完整目录结构和访问权限说明。

## 🗂️ 存储结构

### 公开存储桶

#### `avatars/` - 用户头像
**访问权限**: 公开（无需验证）

**存储路径**:
```
avatars/{user-id}.{extension}
```

**示例**:
- `avatars/user-123.jpg`
- `avatars/user-456.png`

**支持格式**: jpg, jpeg, png, gif, webp

**大小限制**: 5 MB

**访问方式**:
- 直接访问: `http://localhost:9000/avatars/user-123.jpg`
- 无需 Token 验证

---

### 私密存储桶

#### `group-file/` - 群聊文件
**访问权限**: 私密（需要验证群成员身份）

**存储路径**:
```
group-file/
    {group-id}/
        files/      # 文档类型文件
        videos/     # 视频类型文件
        images/     # 图片类型文件
```

**示例**:
- `group-file/group-001/files/document.pdf`
- `group-file/group-001/images/photo.jpg`
- `group-file/group-001/videos/video.mp4`

**访问要求**:
- 必须携带 Access Token
- 必须是群成员
- 通过后端 API 代理访问

---

#### `user-file/` - 用户个人文件
**访问权限**: 私密（需要验证用户身份）

**存储路径**:
```
user-file/
    {user-id}/
        files/      # 文档类型文件
        videos/     # 视频类型文件
        images/     # 图片类型文件
```

**示例**:
- `user-file/user-123/files/resume.pdf`
- `user-file/user-123/images/screenshot.png`
- `user-file/user-123/videos/recording.mp4`

**访问要求**:
- 必须携带 Access Token
- 只能访问自己的文件
- 通过后端 API 代理访问

---

#### `friends-file/` - 好友间聊天文件
**访问权限**: 私密（需要验证好友关系）

**存储路径**:
```
friends-file/
    {conversation-uuid}/
        files/      # 文档类型文件
        videos/     # 视频类型文件
        images/     # 图片类型文件
```

**conversation-uuid 生成规则**:
- 将两个用户ID排序后组合
- 确保双方访问相同的目录
- 格式: `conv-{user-id-1}-{user-id-2}` (ID已排序)

**示例**:
- `friends-file/conv-user-123-user-456/images/photo.jpg`
- `friends-file/conv-user-123-user-456/files/document.pdf`
- `friends-file/conv-user-123-user-456/videos/clip.mp4`

**访问要求**:
- 必须携带 Access Token
- 必须是会话的参与者之一（通过 conversation-uuid 验证）
- 通过后端 API 代理访问

**验证逻辑**:
```rust
// 从 conversation-uuid 提取用户ID
// 检查请求用户是否是其中之一
if conversation_uuid.contains(request_user_id) {
    // 允许访问
} else {
    // 拒绝访问
}
```

---

## 🔐 访问控制策略

### 公开访问（avatars）

- ✅ 无需 Token
- ✅ 直接通过 MinIO URL 访问
- ✅ CDN 缓存友好
- ⚠️ 不要存储敏感信息

### 私密访问（其他所有文件）

**访问流程**:
```
1. 客户端请求文件
   GET /api/friends/files/access?file_url=...
   Headers: Authorization: Bearer {access_token}

2. 后端验证
   - 验证 Access Token
   - 从 URL 解析 conversation-uuid
   - 验证用户是否有权访问该会话
   - 验证好友关系

3. 返回文件
   - 方式 1: 直接返回文件流
   - 方式 2: 生成临时签名URL（推荐）
```

**临时签名 URL**:
```rust
// 生成有效期 5 分钟的临时访问URL
let presigned_url = s3_client.presign_get(
    &file_path,
    Duration::from_secs(300)  // 5分钟
).await?;

// 返回给客户端
// 客户端可在 5 分钟内直接访问该 URL
```

---

## 📦 文件类型和大小限制

### 图片文件 (images/)

**支持格式**: jpg, jpeg, png, gif, webp

**大小限制**:
- 头像: 5 MB
- 聊天图片: 10 MB

### 视频文件 (videos/)

**支持格式**: mp4, avi, mov, mkv

**大小限制**: 100 MB

### 文档文件 (files/)

**支持格式**: pdf, doc, docx, xls, xlsx, ppt, pptx, txt, zip, rar

**大小限制**: 50 MB

---

## 🚀 上传流程

### 好友聊天文件上传

```
步骤 1: 上传文件到 MinIO
POST /api/friends/files/upload
Content-Type: multipart/form-data
Headers: Authorization: Bearer {access_token}

Body:
  friend_id: user-456
  file_type: image | video | file
  file: [binary data]

响应:
{
  "file_url": "http://localhost:9000/friends-file/conv-xxx/images/abc.jpg",
  "file_size": 1024000
}

步骤 2: 发送消息
POST /api/friends/messages

Body:
{
  "receiver_id": "user-456",
  "message_type": "image",
  "message_content": "图片描述（可选）",
  "file_url": "http://localhost:9000/friends-file/conv-xxx/images/abc.jpg"
}
```

---

## 🧹 文件清理策略

### 自动清理规则

1. **孤儿文件清理**: 定期扫描未被消息表引用的文件
2. **已删除消息文件**: 消息被双方都删除后，延迟 30 天删除文件
3. **临时文件清理**: 上传后 24 小时未被消息引用的文件自动删除

### 手动清理

```bash
# 清理超过 30 天的孤儿文件
# 建议通过定时任务执行
```

---

## 📊 存储空间估算

### 示例计算

假设系统有 10,000 个用户：

| 文件类型 | 平均大小 | 每用户数量 | 总空间 |
|---------|---------|----------|--------|
| 头像 | 500 KB | 1 | 5 GB |
| 聊天图片 | 2 MB | 100 | 2 TB |
| 聊天视频 | 20 MB | 10 | 2 TB |
| 聊天文件 | 5 MB | 20 | 1 TB |
| **总计** | - | - | **~5 TB** |

**建议配置**: 至少 10 TB 存储空间

---

## 🔧 MinIO 配置

### 存储桶策略

**avatars 桶（公开读）**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {"AWS": ["*"]},
      "Action": ["s3:GetObject"],
      "Resource": ["arn:aws:s3:::avatars/*"]
    }
  ]
}
```

**其他桶（私密）**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Deny",
      "Principal": {"AWS": ["*"]},
      "Action": ["s3:GetObject"],
      "Resource": [
        "arn:aws:s3:::friends-file/*",
        "arn:aws:s3:::group-file/*",
        "arn:aws:s3:::user-file/*"
      ]
    }
  ]
}
```

---

## 📖 相关文档

- [data.md](./data.md) - 简化的目录结构说明
- [PostgreSQL 数据结构](../PostgreSQL/数据结构说明.md) - 数据库表结构
- [Storage Services README](../src/storage/services/README.md) - 文件上传服务

---

## ⚠️ 安全注意事项

1. **永远不要将私密文件设为公开访问**
2. **临时签名 URL 的有效期不应超过 15 分钟**
3. **所有文件访问必须经过后端验证**
4. **定期审计文件访问日志**
5. **对上传文件进行病毒扫描（可选）**
6. **限制单个用户的存储配额**


