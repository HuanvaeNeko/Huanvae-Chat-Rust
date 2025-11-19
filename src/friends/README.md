# Friends 模块

好友请求与好友关系管理模块，复用 `src/auth` 的认证与中间件，提供提交好友申请、同意/拒绝申请、查询请求与已拥有好友的能力。

## 📂 目录结构

```
src/friends/
  ├─ handlers/         // HTTP 请求处理器层
  ├─ models/           // 请求/响应数据模型
  ├─ services/         // 业务逻辑与数据读写
  └─ mod.rs            // 模块导出
```

## 🔗 路由映射

- `POST /api/friends/requests` → 提交好友申请
- `POST /api/friends/requests/approve` → 同意好友申请
- `POST /api/friends/requests/reject` → 拒绝好友申请
- `GET  /api/friends/requests/sent` → 查看本人发出的待处理申请
- `GET  /api/friends/requests/pending` → 查看本人收到的待处理申请
- `GET  /api/friends` → 查看本人已拥有好友（仅返回 `status=active`）
- `POST /api/friends/remove` → 删除好友（标记结束，不物理删除）

所有上述路由均通过认证中间件，使用 `Extension<AuthContext>` 注入认证信息。

注意：前端请求请勿在 `GET /api/friends` 末尾追加斜杠，否则会出现 404。

## 🧩 数据存储约定（基于 users 表 TEXT 字段）

- `users."user-sent-friend-requests"`：本用户发出的申请列表
- `users."user-pending-friend-requests"`：本用户收到的待处理申请列表
- `users."user-owned-friends"`：本用户已拥有的好友列表（含状态）

记录采用“键值对逗号分隔；记录间分号分隔”的序列化协议，示例：

```
request-id:12345,request-user-id:u1,request-time:2025-01-01T00:00:00Z,status:open;
friend-id:u2,add-time:2025-01-02T12:00:00Z,approve-reason:同意备注;
```

标准键名：

- 申请：`request-id`、`request-user-id`、`request-message`、`request-time`、`status`
- 发出：`request-id`、`sent-to-user-id`、`sent-message`、`sent-time`、`status`
- 好友：`friend-id`、`add-time`、`approve-reason`、`status`

`status` 取值：
- 请求：`open | approved | rejected`
- 好友：`active | ended`
读取规则：好友列表仅返回 `status=active`；删除好友时将记录标记为 `status=ended`，并记录 `remove-time/remove-reason`。

## 🔄 业务流程

1. 提交申请
   - 申请人向目标用户发起请求，记录写入申请人的 `sent` 与目标用户的 `pending`
   - 若双方互相存在 `open` 申请，自动互相通过
2. 同意申请
   - 目标用户同意后，双方的 `sent/pending` 标记为 `approved`，并互相写入 `owned-friends`
3. 拒绝申请
   - 标记双方相关记录为 `rejected`，可选记录拒绝原因
4. 列表查询
   - 仅返回 `status=open` 的 `sent/pending`
   - `owned` 仅返回 `status=active` 的好友条目

## 🗑️ 删除好友

- 路径：`POST /api/friends/remove`
- 请求示例：
```
{
  "user_id": "user-123",
  "friend_user_id": "user-456",
  "remove_time": "2025-11-19T03:00:00Z",
  "remove_reason": "不常联系"
}
```
- 认证：需 `Authorization: Bearer <token>`；处理器校验 `user_id` 与认证上下文一致
- 行为：双向将 `user-owned-friends` 中匹配记录 `status=ended`，写入 `remove-time/remove-reason`
- 幂等：重复调用不会产生额外副作用

## 🏗️ 设计原则

- 职责清晰：Handlers 仅负责 HTTP 交互；Services 负责业务与存储；Models 负责数据结构
- 认证一致：沿用 `src/auth` 的已实现功能与中间件
- 风格一致：模块划分与 README 风格与 `src/auth` 保持一致

## 🔧 初始化与挂载

在 `src/main.rs` 中通过 `create_friend_routes(...)` 挂载至 `/api/friends`，并应用认证中间件。