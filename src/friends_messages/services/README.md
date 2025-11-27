# Services 目录

好友消息的业务逻辑层，处理消息的发送、查询、删除和撤回。

## 📂 文件说明

### `message_service.rs`
用途：消息服务的核心业务逻辑实现。

包含：
- `MessageService` - 消息服务结构体
  - `new(db: PgPool)` - 创建服务实例
  - `generate_conversation_uuid()` - 生成会话UUID
  - `verify_friendship()` - 验证好友关系
  - `send_message()` - 发送消息
  - `get_messages()` - 获取消息列表
  - `delete_message()` - 删除消息
  - `recall_message()` - 撤回消息

## 🔧 核心功能

### 1. 生成会话UUID
```rust
pub fn generate_conversation_uuid(user_id_1: &str, user_id_2: &str) -> String
```
- 将两个用户ID按字母顺序排序
- 组合生成唯一的会话标识：`conv-{user1}-{user2}`
- 保证双方使用相同的会话UUID

### 2. 验证好友关系
```rust
pub async fn verify_friendship(&self, user_id: &str, friend_id: &str) -> Result<bool, AuthError>
```
- 从 `users` 表查询 `user-owned-friends` 字段
- 检查是否包含目标好友ID且状态为 `active`
- 返回验证结果

### 3. 发送消息
```rust
pub async fn send_message(
    &self,
    sender_id: &str,
    receiver_id: &str,
    message_content: &str,
    message_type: &str,
    file_url: Option<String>,
    file_size: Option<i64>,
) -> Result<(String, String), AuthError>
```

流程：
1. 验证双方是否为好友关系
2. 生成消息UUID（`uuid::Uuid::new_v4()`）
3. 生成会话UUID（排序用户ID）
4. 获取服务器UTC时间
5. 插入消息到 `friend-messages` 表
6. 返回消息UUID和发送时间

错误处理：
- 如果不是好友关系，返回 `BadRequest`
- 数据库插入失败，返回 `InternalServerError`

### 4. 获取消息列表
```rust
pub async fn get_messages(
    &self,
    user_id: &str,
    friend_id: &str,
    before_uuid: Option<String>,
    limit: i32,
) -> Result<(Vec<MessageResponse>, bool), AuthError>
```

流程：
1. 验证双方是否为好友关系
2. 生成会话UUID
3. 根据是否提供 `before_uuid` 进行分页查询
4. 过滤已删除的消息（根据查询者身份）
5. 按 `send-time` 降序排列
6. 返回消息列表和是否有更多消息的标记

查询逻辑：
- 如果是发送者：只返回 `is_deleted_by_sender = false` 的消息
- 如果是接收者：只返回 `is_deleted_by_receiver = false` 的消息
- 每次多查询1条用于判断是否有更多消息

### 5. 删除消息（软删除）
```rust
pub async fn delete_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AuthError>
```

流程：
1. 查询消息的发送者和接收者
2. 验证消息是否存在
3. 根据用户身份标记删除：
   - 如果是发送者：设置 `is_deleted_by_sender = true`
   - 如果是接收者：设置 `is_deleted_by_receiver = true`
   - 如果既不是发送者也不是接收者：返回 `Forbidden`

特点：
- 双方独立删除，互不影响
- 不物理删除记录，保留完整聊天历史
- 删除后该用户无法再查看此消息

### 6. 撤回消息
```rust
pub async fn recall_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AuthError>
```

流程：
1. 查询消息的发送者和发送时间
2. 验证消息是否存在
3. 验证是否为发送者（只有发送者可以撤回）
4. 检查是否超过2分钟
5. 同时标记双方已删除：`is_deleted_by_sender = true` 和 `is_deleted_by_receiver = true`

限制：
- 只能撤回自己发送的消息
- 只能撤回2分钟内的消息
- 撤回后双方都无法查看

## 🔐 安全性

1. **好友验证**：所有操作前都验证好友关系
2. **权限检查**：删除和撤回前验证用户权限
3. **时间限制**：撤回操作有2分钟时间窗口
4. **软删除**：保留数据完整性，支持审计

## 🔄 数据库操作

- 使用 `sqlx` 异步数据库操作
- 所有查询使用参数化防止SQL注入
- 使用事务保证数据一致性（隐式）
- 外键约束自动维护引用完整性

## ⚠️ 错误处理

所有方法返回 `Result<T, AuthError>`：
- `AuthError::BadRequest` - 请求参数错误或业务规则违反
- `AuthError::Forbidden` - 权限不足
- `AuthError::InternalServerError` - 数据库操作失败

## 📈 性能优化

1. **索引使用**：
   - `idx-messages-conversation-time` - 会话+时间复合索引（主查询）
   - `idx-messages-sender` - 发送者索引
   - `idx-messages-receiver` - 接收者索引

2. **分页查询**：
   - 使用 `LIMIT` 限制返回数量
   - 基于时间戳的游标分页

3. **查询优化**：
   - 在SQL层面过滤已删除消息
   - 避免返回不必要的字段

