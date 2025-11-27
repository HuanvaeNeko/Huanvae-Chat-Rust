# PostgreSQL 数据库初始化脚本

## 📁 目录说明

此目录包含 PostgreSQL 数据库的初始化 SQL 脚本，按执行顺序命名。

## 📋 脚本列表

### `01-schema.sql`
**用途**: 创建核心数据表和基础结构

**创建的表**:
- `users` - 用户数据表
- `groups` - 群聊数据表
- `user-refresh-tokens` - 刷新Token管理表（多设备登录）
- `token-blacklist` - Token黑名单表
- `user-access-cache` - Access Token缓存表

**创建的视图**:
- `view-users-basic` - 用户基础信息视图
- `view-groups-basic` - 群聊基础信息视图

**创建的触发器**:
- `trigger-update-users-timestamp` - 自动更新users表的updated-at
- `trigger-update-groups-timestamp` - 自动更新groups表的updated-at

---

### `02-add-profile-fields.sql`
**用途**: 添加用户个人资料字段

**新增字段**:
- `user-signature` - 用户个性签名
- `user-avatar-url` - 用户头像URL

**创建的索引**:
- `idx-users-avatar-url` - 头像URL索引

---

### `03-add-friend-messages.sql`
**用途**: 添加好友聊天消息相关表

**创建的表**:
1. **friend-messages** - 好友消息表
   - 存储用户之间的所有聊天消息
   - 支持文本、图片、视频、文件等多种消息类型
   - 使用 conversation-uuid 组织会话
   - 支持软删除（双向独立删除）
   - 所有时间使用 UTC 时间戳

2. **friend-unread-messages** - 未读消息表
   - 存储每个用户与好友的会话信息
   - 记录未读消息计数
   - 缓存最后一条消息内容用于会话列表展示
   - 自动更新时间戳

**创建的索引**:
- `idx-messages-conversation-time` - 会话消息时间复合索引（主查询）
- `idx-messages-sender` - 发送者索引
- `idx-messages-receiver` - 接收者索引
- `idx-messages-send-time` - 发送时间索引
- `idx-messages-conversation` - 会话索引
- `idx-unread-user-updated` - 用户会话列表索引
- `idx-unread-conversation` - 未读会话索引
- `idx-unread-count` - 未读计数索引（部分索引）

**创建的触发器**:
- `trigger-update-unread-timestamp` - 自动更新未读消息表的updated-at

**设计特点**:
- **conversation-uuid 生成规则**: 将两个用户ID排序后组合，确保双方查询同一会话
  - 示例: user-123 和 user-456 → `conv-user-123-user-456`
- **消息软删除**: 双方可独立删除消息，不影响对方查看
- **时间使用 UTC**: 所有时间字段统一使用 UTC，客户端负责转换为本地时间
- **未读计数优化**: 使用独立表存储未读计数，避免频繁扫描消息表

---

## 🚀 执行顺序

PostgreSQL 会按照文件名的字母顺序执行初始化脚本：

1. `01-schema.sql` - 基础表结构
2. `02-add-profile-fields.sql` - 用户资料字段
3. `03-add-friend-messages.sql` - 消息相关表

**注意**: 不要修改文件名前缀的数字顺序，否则可能导致外键约束失败。

---

## 📊 数据库架构总览

### 核心表关系

```
users (用户表)
  ├─→ user-refresh-tokens (刷新Token，多设备)
  ├─→ token-blacklist (Token黑名单)
  ├─→ user-access-cache (Access Token缓存)
  ├─→ friend-messages (好友消息 - 发送者)
  ├─→ friend-messages (好友消息 - 接收者)
  └─→ friend-unread-messages (未读消息)

groups (群聊表)
  └─→ (暂无外键关联)
```

---

## 🔧 使用方法

### 初始化新数据库

```bash
# 进入 PostgreSQL 容器
docker exec -it huanvae-chat-postgres bash

# 连接数据库
psql -U huanvae_admin -d huanvae_chat

# 执行初始化脚本（按顺序）
\i /docker-entrypoint-initdb.d/01-schema.sql
\i /docker-entrypoint-initdb.d/02-add-profile-fields.sql
\i /docker-entrypoint-initdb.d/03-add-friend-messages.sql
```

### 重置数据库

```bash
# 删除所有表（谨慎操作！）
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;

# 重新执行所有初始化脚本
\i /docker-entrypoint-initdb.d/01-schema.sql
\i /docker-entrypoint-initdb.d/02-add-profile-fields.sql
\i /docker-entrypoint-initdb.d/03-add-friend-messages.sql
```

### 验证表结构

```sql
-- 查看所有表
\dt

-- 查看特定表结构
\d "friend-messages"
\d "friend-unread-messages"

-- 查看索引
\di

-- 查看触发器
\dft
```

---

## 📖 相关文档

- [数据结构说明.md](../数据结构说明.md) - 详细的表结构文档
- [MinIO/data.md](../../MinIO/data.md) - 文件存储结构说明

---

## ⚠️ 注意事项

1. **时间字段**: 所有时间字段使用 `TIMESTAMP WITHOUT TIME ZONE`，统一存储 UTC 时间
2. **外键级联**: 所有外键使用 `ON DELETE CASCADE`，删除用户时会级联删除相关数据
3. **索引优化**: 已针对高频查询创建复合索引，注意定期 `ANALYZE` 表以更新统计信息
4. **软删除**: 消息表使用软删除标记，不会物理删除数据
5. **唯一约束**: 未读消息表对 `(user-id, conversation-uuid)` 有唯一约束

---

## 🔄 版本历史

- **2025-01-15**: 创建基础表结构 (01-schema.sql)
- **2025-01-25**: 添加用户资料字段 (02-add-profile-fields.sql)
- **2025-11-27**: 添加好友消息相关表 (03-add-friend-messages.sql)


