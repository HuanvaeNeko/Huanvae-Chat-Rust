# Models 目录

好友消息的数据模型，定义消息的请求、响应和数据库映射结构。

## 📂 文件说明

### `message.rs`
用途：定义数据库消息模型和返回给前端的响应模型。

包含：
- `MessageType` - 消息类型枚举（text/image/video/file）
- `Message` - 数据库消息模型（与 `friend-messages` 表映射）
- `MessageResponse` - 返回给前端的消息响应模型

关键特性：
- 使用 `#[sqlx(rename = "...")]` 将 Rust 字段映射到数据库的连字符列名
- 使用 `#[serde(rename = "...")]` 定义 JSON 序列化格式
- `Message` 包含软删除标记字段（`is_deleted_by_sender`、`is_deleted_by_receiver`）
- `MessageResponse` 不包含删除标记，仅返回必要信息

### `request.rs`
用途：定义客户端发送消息的请求体结构。

包含：
- `SendMessageRequest` - 发送消息请求
  - `receiver_id`: 接收者用户ID
  - `message_content`: 消息内容
  - `message_type`: 消息类型（text/image/video/file）
  - `file_url`: 可选，文件URL（媒体消息时使用）
  - `file_size`: 可选，文件大小（字节）

- `GetMessagesRequest` - 获取消息列表请求（通过查询参数传递）
  - `friend_id`: 好友用户ID
  - `before_uuid`: 可选，分页起点消息UUID
  - `limit`: 可选，返回消息数量（默认50，最大500）

- `DeleteMessageRequest` - 删除消息请求
  - `message_uuid`: 要删除的消息UUID

- `RecallMessageRequest` - 撤回消息请求
  - `message_uuid`: 要撤回的消息UUID

### `response.rs`
用途：定义服务器返回的响应结构。

包含：
- `SendMessageResponse` - 发送消息响应
  - `message_uuid`: 生成的消息UUID
  - `send_time`: 服务器生成的发送时间（UTC，RFC3339格式）

- `GetMessagesResponse` - 获取消息列表响应
  - `messages`: 消息数组
  - `has_more`: 是否还有更多消息

- `DeleteMessageResponse` - 删除消息响应
  - `success`: 是否成功
  - `message`: 提示信息

- `RecallMessageResponse` - 撤回消息响应
  - `success`: 是否成功
  - `message`: 提示信息

## 🧩 约束与约定

- **消息UUID**：由服务器生成（`uuid::Uuid::new_v4()`）
- **会话UUID**：由双方用户ID排序后组合生成（`conv-{user1}-{user2}`）
- **时间格式**：UTC时间，使用 `chrono::DateTime<Utc>`
- **软删除**：消息不会物理删除，通过标记字段实现双方独立删除
- **撤回限制**：只能撤回2分钟内发送的消息，撤回后双方都标记为已删除
- **消息类型**：
  - `text` - 纯文本消息
  - `image` - 图片消息（需要 file_url 和 file_size）
  - `video` - 视频消息（需要 file_url 和 file_size）
  - `file` - 文件消息（需要 file_url 和 file_size）

## 📊 数据库列名映射

| Rust 字段 | 数据库列名 | 类型 |
|-----------|-----------|------|
| message_uuid | message-uuid | TEXT |
| conversation_uuid | conversation-uuid | TEXT |
| sender_id | sender-id | TEXT |
| receiver_id | receiver-id | TEXT |
| message_content | message-content | TEXT |
| message_type | message-type | TEXT |
| file_url | file-url | TEXT |
| file_size | file-size | BIGINT |
| send_time | send-time | TIMESTAMPTZ |
| is_deleted_by_sender | is-deleted-by-sender | BOOLEAN |
| is_deleted_by_receiver | is-deleted-by-receiver | BOOLEAN |

