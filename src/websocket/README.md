# WebSocket 实时通信模块

提供实时消息推送、未读消息通知、已读同步等功能。

## 📂 目录结构

```
src/websocket/
├── handlers/                    # WebSocket 处理器
│   ├── mod.rs
│   ├── routes.rs                # WebSocket 路由
│   ├── state.rs                 # 状态管理
│   └── connection.rs            # 连接建立/消息处理
├── models/
│   ├── mod.rs
│   └── ws_message.rs            # WebSocket 消息协议定义
├── services/
│   ├── mod.rs
│   ├── connection_manager.rs    # 连接管理器（核心）
│   ├── notification_service.rs  # 通知推送服务
│   └── unread_service.rs        # 未读消息服务
├── mod.rs
└── README.md
```

## 🔗 路由映射

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/ws?token=xxx` | WebSocket 连接端点 |
| GET | `/ws/status` | WebSocket 状态查询（调试用） |

## 📡 连接流程

```
1. 用户登录获取 Access Token
2. 建立 WebSocket 连接: ws://host/ws?token=xxx
3. 服务端验证 Token
4. 连接成功，推送未读消息摘要
5. 保持心跳（30秒间隔）
6. 接收实时消息通知
```

## 📝 消息协议

### 服务端 → 客户端

#### 1. 连接成功响应 (connected)

```json
{
  "type": "connected",
  "unread_summary": {
    "friend_unreads": [
      {
        "friend_id": "user123",
        "friend_nickname": "张三",
        "friend_avatar": "http://...",
        "unread_count": 5,
        "last_message_preview": "你好...",
        "last_message_type": "text",
        "last_message_time": "2025-12-04T10:00:00Z"
      }
    ],
    "group_unreads": [
      {
        "group_id": "550e8400-...",
        "group_name": "技术交流群",
        "group_avatar": "http://...",
        "unread_count": 10,
        "last_message_preview": "大家好...",
        "last_message_type": "text",
        "last_sender_nickname": "李四",
        "last_message_time": "2025-12-04T10:05:00Z"
      }
    ],
    "total_count": 15
  }
}
```

#### 2. 新消息通知 (new_message)

```json
{
  "type": "new_message",
  "source_type": "friend",
  "source_id": "user123",
  "message_uuid": "660e8400-...",
  "sender_id": "user123",
  "sender_nickname": "张三",
  "preview": "你好，在吗？",
  "message_type": "text",
  "timestamp": "2025-12-04T10:10:00Z"
}
```

**source_type 取值**:
- `friend` - 好友私信
- `group` - 群聊消息

#### 3. 消息撤回通知 (message_recalled)

```json
{
  "type": "message_recalled",
  "source_type": "friend",
  "source_id": "user123",
  "message_uuid": "660e8400-...",
  "recalled_by": "user123"
}
```

#### 4. 已读同步通知 (read_sync)

> 注意：仅当 `WS_ENABLE_READ_RECEIPT=true` 时发送

```json
{
  "type": "read_sync",
  "source_type": "friend",
  "source_id": "user123",
  "reader_id": "user456",
  "read_at": "2025-12-04T10:15:00Z"
}
```

#### 5. 系统通知 (system_notification)

```json
{
  "type": "system_notification",
  "notification_type": "friend_request",
  "data": {
    "from_user_id": "user789",
    "from_nickname": "王五",
    "message": "请求添加好友"
  }
}
```

**notification_type 取值**:
- `friend_request` - 新好友申请
- `friend_request_approved` - 好友申请已通过
- `friend_request_rejected` - 好友申请被拒绝
- `group_invite` - 群邀请
- `group_join_request` - 入群申请
- `group_join_approved` - 入群申请已通过
- `group_removed` - 被移出群聊
- `group_disbanded` - 群解散
- `group_notice_updated` - 群公告更新

#### 6. 心跳响应 (pong)

```json
{
  "type": "pong",
  "timestamp": "2025-12-04T10:20:00Z"
}
```

#### 7. 错误消息 (error)

```json
{
  "type": "error",
  "code": "invalid_message",
  "message": "Failed to parse message"
}
```

### 客户端 → 服务端

#### 1. 标记已读 (mark_read)

```json
{
  "type": "mark_read",
  "target_type": "friend",
  "target_id": "user123"
}
```

**target_type 取值**:
- `friend` - 好友会话
- `group` - 群聊（target_id 为群 UUID）

#### 2. 心跳 (ping)

```json
{
  "type": "ping"
}
```

#### 3. 订阅在线状态 (subscribe_presence) - 预留

```json
{
  "type": "subscribe_presence",
  "user_ids": ["user123", "user456"]
}
```

## ⚙️ 环境变量配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `WS_ENABLE_READ_RECEIPT` | `true` | 是否启用已读回执功能 |
| `WS_HEARTBEAT_INTERVAL_SECONDS` | `30` | 心跳间隔（秒） |
| `WS_CLIENT_TIMEOUT_SECONDS` | `60` | 客户端超时（秒） |

### 已读回执功能说明

当 `WS_ENABLE_READ_RECEIPT=false` 时：
- ✅ 仍然记录已读状态到数据库
- ❌ 不会向对方发送已读通知
- ❌ 返回的消息中不包含已读状态

这对于注重隐私的应用场景很有用。

## 🔐 安全性

### Token 验证

WebSocket 连接通过 URL 参数传递 Access Token：

```
ws://localhost:8080/ws?token=eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

- 使用与 HTTP API 相同的 JWT 验证逻辑
- Token 过期后连接会被断开
- 每个连接绑定到特定用户和设备

### 权限控制

- 只能收到与自己相关的消息通知
- 标记已读前验证好友/群成员关系
- 多设备同步确保同一用户的所有设备状态一致

## 🏗️ 架构设计

### ConnectionManager

高性能并发连接管理器，支持：
- 同一用户多设备同时在线
- 设备级别的消息路由
- 原子操作保证线程安全
- 使用 DashMap 实现无锁读取

> ⚠️ **重要**：`ConnectionManager` 不实现 `Clone` trait，必须通过 `Arc<ConnectionManager>` 共享。
> 这确保了所有引用指向同一个连接状态，避免状态不一致的问题。
> 参见 `app_state.rs` 中的使用方式：`pub connection_manager: Arc<ConnectionManager>`

```rust
// 向指定用户发送消息（所有设备）
connection_manager.send_to_user("user123", &message);

// 向指定设备发送消息
connection_manager.send_to_device("user123", "device1", &message);

// 向多个用户广播
connection_manager.send_to_users(&["user1", "user2"], &message);

// 向用户的其他设备同步（排除当前设备）
connection_manager.send_to_other_devices("user123", "current_device", &message);
```

### NotificationService

通知推送服务，集成：
- 好友消息通知
- 群聊消息通知
- 消息撤回通知
- 已读同步通知
- 系统通知

### UnreadService

未读消息管理，功能：
- 获取未读摘要
- 增加未读计数
- 批量增加群成员未读
- 标记已读

## 📊 数据库依赖

使用现有的未读消息表：

### friend-unread-messages 表

| 字段 | 说明 |
|------|------|
| user-id | 用户 ID |
| friend-id | 好友 ID |
| unread-count | 未读数量 |
| last-message-content | 最后消息预览 |
| last-message-time | 最后消息时间 |

### group-unread-messages 表

| 字段 | 说明 |
|------|------|
| user-id | 用户 ID |
| group-id | 群 ID |
| unread-count | 未读数量 |
| last-message-content | 最后消息预览 |
| last-sender-id | 最后发送者 |
| last-message-time | 最后消息时间 |

## 📝 使用示例

### JavaScript 客户端

```javascript
// 建立连接
const token = localStorage.getItem('access_token');
const ws = new WebSocket(`ws://localhost:8080/ws?token=${token}`);

// 连接成功
ws.onopen = () => {
  console.log('WebSocket connected');
};

// 接收消息
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch (message.type) {
    case 'connected':
      // 处理未读摘要
      updateUnreadBadges(message.unread_summary);
      break;
      
    case 'new_message':
      // 处理新消息通知
      showNotification(message);
      break;
      
    case 'read_sync':
      // 处理已读同步
      updateReadStatus(message);
      break;
      
    case 'pong':
      // 心跳响应
      break;
  }
};

// 标记已读
function markRead(targetType, targetId) {
  ws.send(JSON.stringify({
    type: 'mark_read',
    target_type: targetType,
    target_id: targetId
  }));
}

// 心跳（可选，服务端会主动发送）
setInterval(() => {
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'ping' }));
  }
}, 30000);
```

### Rust 客户端

```rust
use tokio_tungstenite::connect_async;
use futures_util::{SinkExt, StreamExt};

async fn connect_ws(token: &str) {
    let url = format!("ws://localhost:8080/ws?token={}", token);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // 接收消息
    while let Some(msg) = read.next().await {
        if let Ok(msg) = msg {
            println!("Received: {}", msg);
        }
    }
}
```

## 🔄 集成说明

WebSocket 模块已完成集成到现有消息系统，在发送消息时自动触发实时通知：

1. **好友消息**: `friends_messages/handlers/send_message.rs` 发送消息后调用 `NotificationService::notify_friend_message`
2. **群聊消息**: `group_messages/handlers/send_message.rs` 发送消息后调用 `NotificationService::notify_group_message`

**集成实现位置**：

- `MessagesState` 和 `GroupMessagesState` 通过 `with_notification()` 构造函数注入 `NotificationService`
- `AppState` 中的 `messages_state()` 和 `group_messages_state()` 方法自动使用带通知服务的构造函数

**Nginx 代理配置**：

WebSocket 路由 `/ws` 需要专用的 Nginx 代理配置（已在 `nginx.conf.template` 中配置）：
- 长连接超时：7 天
- WebSocket 升级头自动处理
- 禁用缓冲以支持实时传输

## ✅ 功能清单

| 功能 | 状态 | 说明 |
|------|------|------|
| WebSocket 连接 | ✅ 已实现 | Token 认证 |
| 未读消息摘要 | ✅ 已实现 | 连接时推送 |
| 新消息通知 | ✅ 已实现 | 好友/群聊 |
| 标记已读 | ✅ 已实现 | 清空未读计数 |
| 已读回执 | ✅ 已实现 | 可配置开关 |
| 多设备同步 | ✅ 已实现 | 同一用户多端 |
| 消息撤回通知 | ✅ 已实现 | 实时推送 |
| 系统通知 | ✅ 已实现 | 好友/群事件 |
| 心跳机制 | ✅ 已实现 | 30秒间隔 |
| 在线状态 | 🔄 预留 | 待实现 |

## 📚 相关文档

- [src/README.md](../README.md) - 源码模块总览
- [friends_messages/README.md](../friends_messages/README.md) - 好友消息模块
- [group_messages/README.md](../group_messages/README.md) - 群消息模块
- [config.rs](../config.rs) - 配置管理

