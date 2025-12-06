# Group Messages 群消息模块

群消息功能模块，提供群聊消息的发送、获取、删除和撤回功能。

## 📂 目录结构

```
src/group_messages/
├── handlers/               # HTTP 请求处理器
│   ├── routes.rs           # 路由配置
│   ├── state.rs            # 状态管理
│   ├── send_message.rs     # 发送消息
│   ├── get_messages.rs     # 获取消息列表
│   ├── delete_message.rs   # 删除消息（个人）
│   └── recall_message.rs   # 撤回消息
├── models/                 # 数据模型
│   ├── message.rs          # 消息模型
│   ├── request.rs          # 请求模型
│   └── response.rs         # 响应模型
├── services/               # 业务逻辑
│   └── message_service.rs  # 消息服务
├── mod.rs
└── README.md
```

## 🔗 路由映射

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/group-messages` | 发送群消息 |
| GET | `/api/group-messages?group_id=xxx` | 获取群消息列表 |
| DELETE | `/api/group-messages/delete` | 删除消息（个人） |
| POST | `/api/group-messages/recall` | 撤回消息 |

## 🗄️ 数据库设计

### group-messages 表

| 字段 | 类型 | 说明 |
|------|------|------|
| message-uuid | UUID | 消息唯一标识 |
| group-id | UUID | 群聊ID |
| sender-id | TEXT | 发送者ID |
| message-content | TEXT | 消息内容 |
| message-type | TEXT | 消息类型：text/image/video/file/system |
| file-uuid | VARCHAR(36) | 文件UUID |
| file-url | TEXT | 文件URL |
| file-size | BIGINT | 文件大小 |
| reply-to | UUID | 回复的消息UUID |
| send-time | TIMESTAMPTZ | 发送时间 |
| is-recalled | BOOLEAN | 是否已撤回 |
| recalled-by | TEXT | 撤回操作者ID |

### group-message-deletions 表

记录每个用户对消息的个人删除（不影响其他人看到消息）。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 记录ID |
| message-uuid | UUID | 消息UUID |
| user-id | TEXT | 删除消息的用户ID |
| deleted-at | TIMESTAMPTZ | 删除时间 |

### group-unread-messages 表

群未读消息计数。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 记录ID |
| user-id | TEXT | 用户ID |
| group-id | UUID | 群聊ID |
| unread-count | INTEGER | 未读消息数量 |
| last-message-uuid | UUID | 最后一条消息UUID |
| last-message-content | TEXT | 最后一条消息内容 |

## 📝 API 说明

### 1. 发送群消息

**端点**: `POST /api/group-messages`

**请求体**:
```json
{
  "group_id": "550e8400-e29b-41d4-a716-446655440000",
  "message_content": "Hello, everyone!",
  "message_type": "text",
  "file_uuid": null,
  "file_url": null,
  "file_size": null,
  "reply_to": null
}
```

**响应**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "message_uuid": "660e8400-e29b-41d4-a716-446655440001",
    "send_time": "2025-12-03T10:00:00Z"
  }
}
```

**限制**:
- 必须是群成员
- 不能被禁言

> 📝 **实现说明**：未读消息计数更新由 `NotificationService` 统一处理，
> 通过 `UnreadService.batch_increment_group_unread` 方法批量更新所有群成员的未读计数。
> 这避免了代码重复，保持单一数据源原则。参见 `websocket/services/notification_service.rs`。

### 2. 获取群消息列表

**端点**: `GET /api/group-messages?group_id=xxx&before_time=xxx&limit=50`

**参数**:
- `group_id` (必填) - 群聊ID
- `before_time` (可选) - 时间戳分页，ISO 8601 格式
- `limit` (可选) - 返回条数，默认 50，最大 500

> 💡 **性能优化**: 使用 JOIN 一次性获取消息和发送者信息，消除 N+1 查询问题

**响应**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "messages": [
      {
        "message_uuid": "...",
        "group_id": "...",
        "sender_id": "user_a",
        "sender_nickname": "用户A",
        "sender_avatar_url": "...",
        "message_content": "Hello!",
        "message_type": "text",
        "send_time": "2025-12-03T10:00:00Z",
        "is_recalled": false
      }
    ],
    "has_more": true
  }
}
```

### 3. 删除消息（个人）

**端点**: `DELETE /api/group-messages/delete`

**请求体**:
```json
{
  "message_uuid": "660e8400-e29b-41d4-a716-446655440001"
}
```

**说明**:
- 仅对自己不可见
- 不影响其他群成员

### 4. 撤回消息

**端点**: `POST /api/group-messages/recall`

**请求体**:
```json
{
  "message_uuid": "660e8400-e29b-41d4-a716-446655440001"
}
```

**权限**:
- **发送者**: 只能撤回 2 分钟内发送的消息
- **群主/管理员**: 可以撤回任意消息

**撤回后**:
- 所有人都看到 "[消息已撤回]"
- 文件信息不再返回

## 🔐 权限控制

| 操作 | 群主 | 管理员 | 普通成员 |
|------|-----|-------|---------|
| 发送消息 | ✅ | ✅ | ✅（未禁言） |
| 获取消息 | ✅ | ✅ | ✅ |
| 删除消息（个人） | ✅ | ✅ | ✅ |
| 撤回自己的消息（2分钟内） | ✅ | ✅ | ✅ |
| 撤回任意消息 | ✅ | ✅ | ❌ |

## 📝 使用示例

### 发送文本消息

```bash
curl -X POST "http://localhost:8080/api/group-messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "group_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_content": "大家好！",
    "message_type": "text"
  }'
```

### 发送图片消息

```bash
curl -X POST "http://localhost:8080/api/group-messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "group_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_content": "看看这张图",
    "message_type": "image",
    "file_uuid": "d2f612d5-70b0-4d4e-8779-86cf6aeb2b30"
  }'
```

### 回复消息

```bash
curl -X POST "http://localhost:8080/api/group-messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "group_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_content": "我同意！",
    "message_type": "text",
    "reply_to": "660e8400-e29b-41d4-a716-446655440001"
  }'
```

## ⚡ 性能优化

### 数据库索引

| 索引名 | 字段 | 用途 |
|-------|------|------|
| `idx-group-messages-group-time` | `(group-id, send-time DESC)` | 消息列表分页查询 |
| `idx-group-messages-sender-time` | `(sender-id, send-time DESC)` | JOIN 发送者信息 |

### 查询优化

1. **JOIN 优化**: 一次性获取消息和发送者信息，消除 N+1 查询问题
2. **时间戳分页**: 直接使用 `before_time` 参数，避免子查询
3. **复合索引**: 使用 `(group-id, send-time DESC)` 复合索引优化查询

### 消息归档

- 归档表 `group-messages-archive` 存储历史消息
- 默认归档 30 天前的消息
- 自动定时归档任务（每 24 小时检查一次）

### 消息缓存（可选）

- 缓存表 `group-message-cache` 存储热点群的最近消息
- JSONB 格式存储，支持快速读取
- TTL 过期自动清理

