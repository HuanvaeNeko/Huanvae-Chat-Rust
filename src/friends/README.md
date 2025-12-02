# Friends 模块

好友请求与好友关系管理模块，复用 `src/auth` 的认证与中间件，提供提交好友申请、同意/拒绝申请、查询请求与已拥有好友的能力。

## 📂 目录结构

```
src/friends/
  ├─ handlers/         // HTTP 请求处理器层
  │   ├─ routes.rs     // 路由定义
  │   ├─ create_request.rs
  │   ├─ approve_request.rs
  │   ├─ reject_request.rs
  │   ├─ list_sent.rs
  │   ├─ list_pending.rs
  │   ├─ list_owned.rs
  │   └─ remove_friend.rs
  ├─ models/           // 请求/响应数据模型
  │   ├─ request.rs    // 请求体模型
  │   └─ list.rs       // 列表响应模型
  ├─ services/         // 业务逻辑与数据读写
  │   └─ friends_service.rs
  └─ mod.rs            // 模块导出
```

## 🔗 路由映射

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/friends/requests` | 提交好友申请 |
| POST | `/api/friends/requests/approve` | 同意好友申请 |
| POST | `/api/friends/requests/reject` | 拒绝好友申请 |
| GET | `/api/friends/requests/sent` | 查看本人发出的待处理申请 |
| GET | `/api/friends/requests/pending` | 查看本人收到的待处理申请 |
| GET | `/api/friends` | 查看本人已拥有好友 |
| POST | `/api/friends/remove` | 删除好友 |

所有路由均通过认证中间件保护，使用 `Extension<AuthContext>` 注入认证信息。

## 🗄️ 数据库设计（独立关系表）

### friendships 表（好友关系）

存储双向好友关系，每对好友有两条记录（A→B 和 B→A）。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | TEXT | 用户 ID |
| friend_id | TEXT | 好友 ID |
| status | TEXT | 状态: `active`（有效）、`ended`（已删除）|
| remark | TEXT | 好友备注 |
| add_time | TIMESTAMPTZ | 添加时间 |
| end_time | TIMESTAMPTZ | 结束时间（删除时） |
| end_reason | TEXT | 结束原因 |

**唯一约束**: `(user_id, friend_id)`

### friend_requests 表（好友请求）

存储好友申请记录。

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| from_user_id | TEXT | 申请人 |
| to_user_id | TEXT | 被申请人 |
| message | TEXT | 申请消息（**支持特殊字符**） |
| status | TEXT | 状态: `pending`、`approved`、`rejected` |
| reject_reason | TEXT | 拒绝原因 |
| created-at | TIMESTAMPTZ | 创建时间 |
| updated-at | TIMESTAMPTZ | 更新时间 |

## ✅ 特殊字符支持

新的数据库设计解决了旧版 TEXT 字段解析器的漏洞：

- ✅ 分号 `;` - 安全
- ✅ 冒号 `:` - 安全
- ✅ 逗号 `,` - 安全
- ✅ 任意 Unicode 字符 - 安全

用户昵称和消息内容可以包含任意字符，不会导致数据损坏。

## 🔄 业务流程

### 1. 提交申请

- 申请人向目标用户发起请求
- 在 `friend_requests` 表插入一条 `pending` 记录
- **自动互通过**: 若对方已有待处理的申请，自动双向同意并建立好友关系

### 2. 同意申请

- 更新 `friend_requests` 状态为 `approved`
- 在 `friendships` 表插入双向记录

### 3. 拒绝申请

- 更新 `friend_requests` 状态为 `rejected`
- 可选记录拒绝原因

### 4. 删除好友

- 更新双方 `friendships` 记录状态为 `ended`
- 记录删除时间和原因
- 软删除，保留历史记录

## 📝 API 示例

### 提交好友请求

```bash
curl -X POST http://localhost:8080/api/friends/requests \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user_a",
    "target_user_id": "user_b",
    "reason": "你好;这是:测试,消息!",
    "request_time": "2025-12-02T10:00:00Z"
  }'
```

### 同意好友请求

```bash
curl -X POST http://localhost:8080/api/friends/requests/approve \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user_b",
    "applicant_user_id": "user_a",
    "approved_time": "2025-12-02T10:01:00Z"
  }'
```

### 查看好友列表

```bash
curl -X GET http://localhost:8080/api/friends \
  -H "Authorization: Bearer <token>"
```

响应示例：
```json
{
  "items": [
    {
      "friend_id": "user_b",
      "friend_nickname": "用户B:特殊;字符",
      "add_time": "2025-12-02T11:05:01.202820+00:00",
      "approve_reason": null
    }
  ]
}
```

## 🏗️ 设计原则

- **独立表结构**: 使用标准化的关系表替代 TEXT 字段存储
- **安全性**: 消除特殊字符解析漏洞
- **性能**: 支持索引查询，无需全表解析
- **认证一致**: 复用 `src/auth` 的认证中间件
- **软删除**: 保留历史记录，支持数据审计

## 🔧 初始化与挂载

在 `src/main.rs` 中通过 `create_friend_routes(...)` 挂载至 `/api/friends`，并应用认证中间件。

数据库表在 `PostgreSQL/init/07_friendships_tables.sql` 中定义。
