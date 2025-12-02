# 好友消息功能 API 文档

## 概述

好友消息模块提供用户之间的实时消息收发功能，支持文本、图片、视频、文件等多种消息类型。所有消息相关的 API 均需要通过 Access Token 鉴权。

## 模块结构

```
src/friends_messages/
├── handlers/           # HTTP 请求处理器
│   ├── send_message.rs      # 发送消息
│   ├── get_messages.rs      # 获取消息列表
│   ├── delete_message.rs    # 删除消息
│   ├── recall_message.rs    # 撤回消息
│   ├── state.rs             # 状态管理
│   ├── routes.rs            # 路由配置
│   └── mod.rs
├── models/             # 数据模型
│   ├── message.rs           # 消息模型
│   ├── request.rs           # 请求模型
│   ├── response.rs          # 响应模型
│   └── mod.rs
├── services/           # 业务逻辑
│   ├── message_service.rs   # 消息服务
│   └── mod.rs
└── mod.rs
```

## API 端点

### 1. 发送消息

**端点**: `POST /api/messages`

**需要鉴权**: ✅ 是

**请求体**:
```json
{
  "receiver_id": "user-456",
  "message_content": "你好，这是一条测试消息",
  "message_type": "text",
  "file_url": null,      // 可选，媒体消息时填写
  "file_size": null      // 可选，文件大小（字节）
}
```

**消息类型**:
- `text` - 文本消息
- `image` - 图片消息
- `video` - 视频消息
- `file` - 文件消息

**响应示例**:
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
  "send_time": "2025-11-27T10:30:00Z"
}
```

**错误响应**:
- `400` - 参数验证失败或不是好友关系
- `401` - Token 无效
- `500` - 服务器内部错误

---

### 2. 获取消息列表

**端点**: `GET /api/messages?friend_id={friend_id}&before_uuid={uuid}&limit={limit}`

**需要鉴权**: ✅ 是

**查询参数**:
- `friend_id` (必填) - 好友的用户ID
- `before_uuid` (可选) - 从这条消息之前查询（分页）
- `limit` (可选) - 返回条数，默认 50，最大 500

**响应示例**:
```json
{
  "messages": [
    {
      "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
      "sender_id": "user-123",
      "receiver_id": "user-456",
      "message_content": "你好",
      "message_type": "text",
      "file_url": null,
      "file_size": null,
      "send_time": "2025-11-27T10:30:00Z"
    }
  ],
  "has_more": false
}
```

**说明**:
- 消息按 `send_time` 倒序排列（最新的在前）
- 已删除的消息不会返回（根据当前用户的删除标记过滤）
- `has_more` 表示是否还有更多消息

---

### 3. 删除消息（软删除）

**端点**: `DELETE /api/messages/delete`

**需要鉴权**: ✅ 是

**请求体**:
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应示例**:
```json
{
  "success": true,
  "message": "消息已删除"
}
```

**说明**:
- 采用软删除机制，根据用户身份标记 `is_deleted_by_sender` 或 `is_deleted_by_receiver`
- 发送者删除后，接收者仍能看到消息
- 接收者删除后，发送者仍能看到消息
- 双方独立删除，互不影响

---

### 4. 撤回消息

**端点**: `POST /api/messages/recall`

**需要鉴权**: ✅ 是

**请求体**:
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
}
```

**响应示例**:
```json
{
  "success": true,
  "message": "消息已撤回"
}
```

**限制**:
- ⏰ **只能撤回 2 分钟内发送的消息**
- 🔒 **只有发送者可以撤回**
- 🗑️ **撤回后双方都看不到消息**（同时标记 `is_deleted_by_sender` 和 `is_deleted_by_receiver`）

**错误响应**:
- `400` - 消息超过 2 分钟或消息不存在
- `403` - 只有发送者可以撤回

---

## 数据库表结构

### friend-messages 表

| 字段名 | 类型 | 说明 |
|--------|------|------|
| message-uuid | TEXT PRIMARY KEY | 消息唯一标识 |
| conversation-uuid | TEXT | 会话标识 |
| sender-id | TEXT | 发送者ID |
| receiver-id | TEXT | 接收者ID |
| message-content | TEXT | 消息内容 |
| message-type | TEXT | 消息类型 |
| file-url | TEXT | 文件URL（可选） |
| file-size | BIGINT | 文件大小（可选） |
| send-time | TIMESTAMP | 发送时间（UTC） |
| is-deleted-by-sender | BOOLEAN | 发送者是否删除 |
| is-deleted-by-receiver | BOOLEAN | 接收者是否删除 |

**索引**:
- `idx-messages-conversation-time`: (conversation-uuid, send-time DESC) - 主查询索引
- `idx-messages-sender`: (sender-id)
- `idx-messages-receiver`: (receiver-id)
- `idx-messages-send-time`: (send-time DESC)

---

## 业务逻辑

### 会话 UUID 生成规则

使用 `common` 模块的公共函数：

```rust
use crate::common::generate_conversation_uuid;

let conversation_uuid = generate_conversation_uuid(user_id_1, user_id_2);
```

**示例**:
- `user-123` 和 `user-456` → `conv-user-123-user-456`
- `user-456` 和 `user-123` → `conv-user-123-user-456` (相同)

### 好友关系验证

使用 `friends::services` 模块的公共函数：

```rust
use crate::friends::services::verify_friendship;

let is_friend = verify_friendship(&db, user_id, friend_id).await?;
```

发送消息前必须验证双方是否为好友：
1. 查询 `friendships` 表
2. 检查是否存在 `user_id` 和 `friend_id` 的记录
3. 验证 `status = 'active'`

### 软删除机制

| 操作 | is_deleted_by_sender | is_deleted_by_receiver | 发送者可见 | 接收者可见 |
|------|---------------------|----------------------|----------|----------|
| 未删除 | false | false | ✅ | ✅ |
| 发送者删除 | true | false | ❌ | ✅ |
| 接收者删除 | false | true | ✅ | ❌ |
| 撤回（双删） | true | true | ❌ | ❌ |

### 撤回规则

```rust
// 检查是否超过2分钟
let now = Utc::now();
let duration = now.signed_duration_since(send_time);
if duration.num_minutes() > 2 {
    return Err("消息发送超过2分钟，无法撤回");
}
```

---

## 安全性

### 鉴权机制

- ✅ 所有 API 均通过 `auth_guard` 中间件验证 Access Token
- ✅ 使用 `Extension<AuthContext>` 注入用户信息
- ✅ 每次操作前验证好友关系

### 权限控制

- ✅ 只能向好友发送消息
- ✅ 只能查看与好友之间的消息
- ✅ 只能删除涉及自己的消息
- ✅ 只有发送者可以撤回消息

---

## 使用示例

### 完整流程示例

```bash
# 1. 用户登录获取 Token
TOKEN=$(curl -s -X POST "http://localhost:8080/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "password": "password123"
  }' | jq -r '.access_token')

# 2. 发送文本消息
curl -X POST "http://localhost:8080/api/messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "receiver_id": "bob",
    "message_content": "你好，Bob！",
    "message_type": "text"
  }'

# 3. 获取消息列表
curl -X GET "http://localhost:8080/api/messages?friend_id=bob&limit=50" \
  -H "Authorization: Bearer $TOKEN"

# 4. 删除消息
curl -X DELETE "http://localhost:8080/api/messages/delete" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
  }'

# 5. 撤回消息（2分钟内）
curl -X POST "http://localhost:8080/api/messages/recall" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

---

## 错误处理

所有错误统一使用 `AppError` 类型（`crate::common::AppError`）：

```rust
pub enum AppError {
    Unauthorized,              // 401 - 未授权访问
    InvalidCredentials,        // 401 - 用户名或密码错误
    InvalidToken,              // 401 - Token 无效或已过期
    TokenRevoked,              // 401 - Token 已被撤销
    Forbidden,                 // 403 - 权限不足
    NotFound(String),          // 404 - 资源不存在
    BadRequest(String),        // 400 - 请求参数错误
    ValidationError(String),   // 400 - 验证错误
    Conflict(String),          // 409 - 资源冲突
    Internal,                  // 500 - 内部服务器错误
    Database(String),          // 500 - 数据库错误
    Storage(String),           // 500 - 存储服务错误
}
```

---

## 性能优化

### 数据库索引

- ✅ 会话+时间复合索引，优化消息列表查询
- ✅ 发送者/接收者索引，支持按用户查询
- ✅ 时间索引，支持时间范围查询

### 查询优化

- ✅ 使用 `LIMIT` 限制返回数量
- ✅ 分页查询避免一次性加载大量数据
- ✅ 软删除过滤在 SQL 层完成

### 扩展性考虑

- 🔄 未来可添加消息已读状态
- 🔄 未来可添加消息搜索功能
- 🔄 未来可添加消息统计（未读数）
- 🔄 未来可添加 WebSocket 实时推送

---

## 测试

完整测试已集成到 `test_all_features.sh`：

```bash
# 运行完整测试
cd /home/huanwei/Huanvae-Chat-Rust
./test_all_features.sh
```

测试覆盖:
- ✅ 发送消息（文本）
- ✅ 获取消息列表
- ✅ 分页查询
- ✅ 删除消息（软删除）
- ✅ 撤回消息（2分钟内）
- ✅ 双向独立删除验证
- ✅ 非好友权限验证
- ✅ Token 鉴权

---

## 已知限制

1. **消息类型**: 当前仅支持文本消息，图片/视频/文件消息需要先实现文件上传功能
2. **实时推送**: 当前为 HTTP 轮询模式，未实现 WebSocket 实时推送
3. **消息搜索**: 未实现消息内容全文搜索
4. **未读计数**: 未实现未读消息统计（已规划 `friend-unread-messages` 表）
5. **消息已读**: 未实现消息已读回执功能

---

## 下一步计划

- [ ] 实现文件上传服务（支持图片/视频/文件消息）
- [ ] 实现 WebSocket 实时消息推送
- [ ] 实现未读消息计数功能
- [ ] 实现消息已读回执
- [ ] 实现消息搜索功能
- [ ] 添加消息统计和分析

