# Handlers 目录

好友消息的HTTP请求处理器层，负责接收客户端请求、调用服务层逻辑、返回响应。

## 📂 文件说明

### `state.rs`
用途：定义消息处理器的共享状态。

包含：
- `MessageState` - 消息状态结构体
  - `db: PgPool` - 数据库连接池
  - `new(db: PgPool)` - 创建状态实例

作用：
- 在所有处理器之间共享数据库连接
- 通过 `State<MessageState>` 注入到处理器函数

### `send_message.rs`
用途：处理发送消息的HTTP请求。

路由：`POST /api/messages`

请求头：
- `Authorization: Bearer <access_token>` - 必需

请求体：
```json
{
  "receiver_id": "user123",
  "message_content": "你好",
  "message_type": "text",
  "file_url": null,
  "file_size": null
}
```

响应：
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
  "send_time": "2025-11-27T03:00:00Z"
}
```

处理流程：
1. 通过 `Extension<AuthContext>` 获取认证用户信息
2. 验证请求体（`SendMessageRequest`）
3. 调用 `MessageService::send_message()`
4. 返回消息UUID和发送时间

### `get_messages.rs`
用途：处理获取消息列表的HTTP请求。

路由：`GET /api/messages`

请求头：
- `Authorization: Bearer <access_token>` - 必需

查询参数：
- `friend_id` - 必需，好友用户ID
- `before_uuid` - 可选，分页起点消息UUID
- `limit` - 可选，返回数量（默认50，最大500）

示例：
```
GET /api/messages?friend_id=user456&limit=50
GET /api/messages?friend_id=user456&before_uuid=550e8400-e29b-41d4-a716-446655440000&limit=50
```

响应：
```json
{
  "messages": [
    {
      "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
      "sender_id": "user123",
      "receiver_id": "user456",
      "message_content": "你好",
      "message_type": "text",
      "file_url": null,
      "file_size": null,
      "send_time": "2025-11-27T03:00:00Z"
    }
  ],
  "has_more": false
}
```

处理流程：
1. 通过 `Extension<AuthContext>` 获取认证用户信息
2. 解析查询参数（`Query<GetMessagesRequest>`）
3. 验证并限制 `limit` 值（最大500）
4. 调用 `MessageService::get_messages()`
5. 返回消息列表和分页信息

### `delete_message.rs`
用途：处理删除消息的HTTP请求（软删除）。

路由：`DELETE /api/messages/delete`

请求头：
- `Authorization: Bearer <access_token>` - 必需

请求体：
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
}
```

响应：
```json
{
  "success": true,
  "message": "消息删除成功"
}
```

处理流程：
1. 通过 `Extension<AuthContext>` 获取认证用户信息
2. 验证请求体（`DeleteMessageRequest`）
3. 调用 `MessageService::delete_message()`
4. 返回成功响应

特点：
- 软删除，不物理删除记录
- 发送者和接收者可以独立删除
- 删除后当前用户无法再查看此消息

### `recall_message.rs`
用途：处理撤回消息的HTTP请求。

路由：`POST /api/messages/recall`

请求头：
- `Authorization: Bearer <access_token>` - 必需

请求体：
```json
{
  "message_uuid": "550e8400-e29b-41d4-a716-446655440000"
}
```

响应：
```json
{
  "success": true,
  "message": "消息撤回成功"
}
```

处理流程：
1. 通过 `Extension<AuthContext>` 获取认证用户信息
2. 验证请求体（`RecallMessageRequest`）
3. 调用 `MessageService::recall_message()`
4. 返回成功响应

限制：
- 只能撤回自己发送的消息
- 只能撤回2分钟内的消息
- 撤回后双方都无法查看

### `routes.rs`
用途：定义并注册消息相关的路由。

包含：
- `create_message_routes()` - 创建路由函数
  - 参数：`message_state: MessageState`, `auth_state: AuthState`
  - 返回：`Router` - 路由实例

路由映射：
```
POST   /api/messages         -> send_message_handler
GET    /api/messages         -> get_messages_handler
DELETE /api/messages/delete  -> delete_message_handler
POST   /api/messages/recall  -> recall_message_handler
```

中间件：
- 所有路由都使用 `auth_guard` 认证中间件
- 自动注入 `AuthContext` 到处理器

## 🔐 认证与授权

所有处理器都通过 `Extension<AuthContext>` 获取认证信息：
- `auth.user_id` - 当前登录用户ID
- `auth.email` - 用户邮箱
- `auth.device_id` - 设备ID

中间件验证：
1. 检查 `Authorization` 请求头
2. 验证 Access Token 有效性
3. 检查 Token 是否在黑名单
4. 将用户信息注入到请求扩展

## 📝 错误处理

所有处理器返回 `Result<Json<T>, AuthError>`：
- 成功：返回 `200 OK` 和 JSON 响应体
- 失败：返回对应的HTTP状态码和错误信息
  - `400 Bad Request` - 请求参数错误或业务规则违反
  - `401 Unauthorized` - 未认证或Token无效
  - `403 Forbidden` - 权限不足
  - `500 Internal Server Error` - 服务器内部错误

错误响应格式：
```json
{
  "error": "错误描述",
  "status": 400
}
```

## 🔄 请求流程

```
客户端请求
    ↓
认证中间件 (auth_guard)
    ↓
处理器函数 (handler)
    ↓
服务层 (MessageService)
    ↓
数据库 (PostgreSQL)
    ↓
响应返回
```

## 📊 状态管理

使用 Axum 的 `State` 模式：
```rust
State(state): State<MessageState>  // 消息状态（数据库连接）
Extension(auth): Extension<AuthContext>  // 认证上下文（中间件注入）
```

## 🎯 最佳实践

1. **参数验证**：在处理器层进行基本验证，业务规则在服务层验证
2. **错误传播**：使用 `?` 操作符传播错误，统一在中间件处理
3. **类型安全**：使用强类型结构体接收请求和返回响应
4. **异步处理**：所有处理器都是异步函数，使用 `async/await`
5. **依赖注入**：通过 `State` 和 `Extension` 注入依赖

