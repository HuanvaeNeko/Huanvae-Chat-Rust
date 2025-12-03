# PostgreSQL 数据库初始化脚本

## 📁 目录说明

此目录包含 PostgreSQL 数据库的初始化 SQL 脚本，按功能模块组织，便于维护和理解。

## 📋 脚本列表

### 核心表结构

#### `01_core_tables.sql`
**用途**: 创建核心数据表和基础结构

**创建的表**:
- `users` - 用户数据表
- `groups` - 群聊数据表

**其他内容**:
- 基础索引（8个）
- 触发器函数 `update_timestamp()`
- 用户和群聊的更新触发器
- 基础视图（`view-users-basic`, `view-groups-basic`）

---

### 认证系统

#### `02_auth_system.sql`
**用途**: 创建认证和Token管理相关表

**创建的表**:
- `user-refresh-tokens` - 刷新Token管理表（多设备登录）
- `token-blacklist` - Token黑名单表
- `user-access-cache` - Access Token缓存表

**索引**: 10个索引，支持快速Token验证和设备管理

---

### 好友系统

#### `03_friend_system.sql`
**用途**: 创建好友关系和消息相关表

**创建的表**:
- `friendships` - 好友关系表
- `friend-requests` - 好友请求表
- `friend-messages` - 好友消息表
- `friend-unread-messages` - 未读消息表

**其他内容**:
- 18个索引（含部分索引）
- 2个触发器（自动更新时间戳）
- 表和字段注释

**设计特点**:
- **conversation-uuid 生成规则**: 将两个用户ID排序后组合，确保双方查询同一会话
  - 示例: user-123 和 user-456 → `conv-user-123-user-456`
- **消息软删除**: 双方可独立删除消息，不影响对方查看
- **时间使用 UTC**: 所有时间字段统一使用 UTC，客户端负责转换为本地时间
- **未读计数优化**: 使用独立表存储未读计数，避免频繁扫描消息表

---

### 文件系统

#### `04_file_system.sql`
**用途**: 创建文件存储和管理相关表

**创建的表**:
- `file-records` - 文件记录表
- `file-uuid-mapping` - UUID映射表（去重核心）
- `file-access-permissions` - 文件访问权限表
- `user-storage-quotas` - 用户存储配额表

**其他内容**:
- 10个索引（含部分索引）
- 触发器：文件完成时自动更新配额
- 表和字段注释

**设计特点**:
- **UUID映射去重**: 跨用户文件去重，相同文件只存储一份
- **权限控制**: 基于权限表的灵活访问控制
- **软删除**: 使用 `revoked-at` 实现权限撤销

---

### 性能优化

#### `05_indexes.sql`
**用途**: 补充复合索引，提升查询性能

**包含索引**:
- file-records 表: 4个复合索引
- friend-messages 表: 4个复合索引
- token-blacklist 表: 2个复合索引
- file-access-permissions 表: 2个复合索引
- user-access-cache 表: 1个复合索引
- friendships 表: 1个复合索引

**总计**: 14个复合/部分索引

**说明**: 部分索引（Partial Index）仅索引满足特定条件的行，可以减少索引大小并提升查询性能。

---

## 🚀 执行顺序

PostgreSQL 会按照文件名的字母顺序执行初始化脚本：

1. `01_core_tables.sql` - 核心表结构（必须最先）
2. `02_auth_system.sql` - 认证系统
3. `03_friend_system.sql` - 好友系统
4. `04_file_system.sql` - 文件系统
5. `05_indexes.sql` - 补充索引

**重要**: 请保持文件名前缀的数字顺序，否则可能导致外键约束失败。

---

## 📊 数据库架构总览

### 核心表关系

```
users (用户表)
  ├─→ user-refresh-tokens (刷新Token)
  ├─→ token-blacklist (Token黑名单)
  ├─→ user-access-cache (Access Token缓存)
  ├─→ friendships (好友关系)
  ├─→ friend-requests (好友请求)
  ├─→ friend-messages (好友消息)
  ├─→ friend-unread-messages (未读消息)
  ├─→ file-records (文件记录)
  └─→ user-storage-quotas (存储配额)

groups (群聊表)
  └─→ (暂无外键关联，待迁移)

file-uuid-mapping (UUID映射)
  └─→ file-access-permissions (文件权限)
```

### 表统计

| 模块 | 表数量 | 索引数量 | 触发器数量 |
|-----|--------|---------|-----------|
| 核心表 | 2 | 8 | 2 |
| 认证系统 | 3 | 10 | 0 |
| 好友系统 | 4 | 18 | 2 |
| 文件系统 | 4 | 10 | 1 |
| 补充索引 | - | 14 | - |
| **总计** | **13** | **60** | **5** |

---

## 🔧 使用方法

### 初始化新数据库

```bash
# 进入 PostgreSQL 容器
docker exec -it huanvae-postgres bash

# 连接数据库
psql -U postgres -d huanvae_chat

# 执行初始化脚本（按顺序）
\i /docker-entrypoint-initdb.d/01_core_tables.sql
\i /docker-entrypoint-initdb.d/02_auth_system.sql
\i /docker-entrypoint-initdb.d/03_friend_system.sql
\i /docker-entrypoint-initdb.d/04_file_system.sql
\i /docker-entrypoint-initdb.d/05_indexes.sql
```

### 重置数据库

```bash
# 删除所有表（谨慎操作！）
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;

# 重新执行所有初始化脚本
\i /docker-entrypoint-initdb.d/01_core_tables.sql
\i /docker-entrypoint-initdb.d/02_auth_system.sql
\i /docker-entrypoint-initdb.d/03_friend_system.sql
\i /docker-entrypoint-initdb.d/04_file_system.sql
\i /docker-entrypoint-initdb.d/05_indexes.sql
```

### 验证表结构

```sql
-- 查看所有表
\dt

-- 查看特定表结构
\d "users"
\d "friend-messages"
\d "file-uuid-mapping"

-- 查看索引
\di

-- 查看触发器
\dft

-- 查看视图
\dv
```

---

## 📖 相关文档

- [数据结构说明.md](../数据结构说明.md) - 详细的表结构文档
- [MinIO/data.md](../../MinIO/data.md) - 文件存储结构说明

---

## ⚠️ 注意事项

1. **时间字段**: 所有时间字段使用 `TIMESTAMPTZ`（带时区）或 `TIMESTAMP`（不带时区），统一存储 UTC 时间
2. **外键级联**: 所有外键使用 `ON DELETE CASCADE`，删除用户时会级联删除相关数据
3. **索引优化**: 已针对高频查询创建复合索引，注意定期 `ANALYZE` 表以更新统计信息
4. **软删除**: 消息和权限表使用软删除标记，不会物理删除数据
5. **唯一约束**: 好友关系、未读消息表有唯一约束，防止重复记录

---

## 🔄 版本历史

- **2025-01-15**: 创建基础表结构（原01-schema.sql）
- **2025-11-25**: 添加用户资料字段（原02-add-profile-fields.sql）
- **2025-11-27**: 添加好友消息相关表（原03-add-friend-messages.sql）
- **2025-12-02**: 添加文件存储系统表（原05-07）
- **2025-12-03**: 按功能模块重构，删除废弃表（v2.0）

---

## 🎯 迁移说明

### 从旧版本迁移

如果你的数据库使用的是旧版本的初始化脚本（01-schema.sql, 02-add-profile-fields.sql等），可以通过以下方式迁移：

1. **备份现有数据**
   ```bash
   pg_dump -U postgres huanvae_chat > backup.sql
   ```

2. **创建新数据库**
   ```bash
   createdb -U postgres huanvae_chat_new
   ```

3. **执行新脚本**
   ```bash
   psql -U postgres -d huanvae_chat_new -f 01_core_tables.sql
   psql -U postgres -d huanvae_chat_new -f 02_auth_system.sql
   # ... 执行其他脚本
   ```

4. **迁移数据**（根据实际情况编写数据迁移脚本）

### 模块化的优势

- ✅ **清晰的结构**: 按功能模块组织，易于理解和维护
- ✅ **独立性**: 每个模块可以独立查看和修改
- ✅ **统一命名**: 使用下划线命名，编号连续
- ✅ **便于扩展**: 新增功能只需添加新的模块文件

---

## 👥 贡献

在修改数据库结构时，请遵循以下原则：

1. 保持模块化组织，功能相关的表放在同一个文件中
2. 更新此 README 文档，说明你的修改
3. 在 `数据结构说明.md` 中同步更新表结构文档
4. 为新增的表和字段添加注释（COMMENT ON）
5. 创建必要的索引以优化查询性能
