# Handlers 目录

HTTP 请求处理器层，负责接收和响应好友相关的 HTTP 请求。

## 📂 文件说明

### `routes.rs`
用途: 路由配置与组装。将读/写端点分别挂载，并应用认证中间件。

### `create_request.rs`
用途: 处理提交好友申请请求。
主要功能:
- 校验请求中的 `user_id` 与认证上下文一致
- 写入申请人的 `user-sent-friend-requests` 与目标用户的 `user-pending-friend-requests`
- 检查是否存在对向开放申请，必要时触发自动互通过

### `approve_request.rs`
用途: 处理同意好友申请请求。
主要功能:
- 标记双方相关 `sent/pending` 为 `approved`
- 将双方互加入 `user-owned-friends`

### `reject_request.rs`
用途: 处理拒绝好友申请请求。
主要功能:
- 标记双方相关 `sent/pending` 为 `rejected`
- 可选记录拒绝原因

### `list_sent.rs`
用途: 查询本人发出的待处理申请列表。
返回: 仅包含 `status = open` 的记录。

### `list_pending.rs`
用途: 查询本人收到的待处理申请列表。
返回: 仅包含 `status = open` 的记录。

### `list_owned.rs`
用途: 查询本人已拥有的好友列表。
返回: 仅返回 `status=active` 的好友条目（删除后的 `status=ended` 不再显示）。

注意：路由为 `GET /api/friends`（无尾斜杠）。

### `remove_friend.rs`
用途: 处理删除好友请求（标记结束，不物理删除）。
主要功能:
- 双向将 `user-owned-friends` 中匹配的记录标记为 `status=ended`
- 记录 `remove-time` 与可选 `remove-reason`
路由: `POST /api/friends/remove`
认证: 通过中间件 `auth_guard` 注入 `AuthContext`；请求体的 `user_id` 必须与认证用户匹配

## 🔄 调用流程

```
客户端请求
    ↓
路由匹配 (routes.rs)
    ↓
认证中间件注入 (auth_guard + Extension<AuthContext>)
    ↓
Handler 处理 (create/approve/reject/list)
    ↓
调用 Services 层执行业务逻辑
    ↓
读取/更新 users 表 TEXT 字段
    ↓
返回 HTTP 响应
```