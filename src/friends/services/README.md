# Services 目录

业务服务层，封装好友申请、批准/拒绝、列表查询等核心逻辑与数据读写。

## 📂 文件说明

### `friends_service.rs`
用途: 提交/同意/拒绝好友申请的业务实现。
主要功能:
- `submit_request`：写入申请人的 `sent` 与目标用户的 `pending`；如检测到对向开放申请，调用 `auto_approve` 自动互通过
- `approve_request`：调用 `manual_approve` 标记 `sent/pending=approved` 并互相加入 `owned-friends`（新增好友记录显式 `status=active`）
- `reject_request`：将双方相关 `sent/pending` 标记为 `rejected`，可选记录拒绝原因
- `remove_friend`：删除好友（标记结束）。双向将 `user-owned-friends` 中匹配记录设置 `status=ended`，并记录 `remove-time/remove-reason`
- `manual_approve`：执行同意操作的具体读写与序列化
- `auto_approve`：在双方互相申请的场景下自动触发同意

### `text_kv.rs`
用途: TEXT 字段序列化协议的解析与操作工具。
主要功能:
- `parse_records`：将 `key:value` 逗号分隔、记录分号分隔的文本解析为 `Vec<HashMap<String,String>>`
- `serialize_records`：将记录集合序列化回文本
- `append_record`：追加新记录
- `set_status`：按条件更新记录的 `status = approved|rejected`

### `mod.rs`
用途: 模块导出，便于 handlers 引用。

## 🧩 数据协议与键名

- 申请（pending）：`request-id`、`request-user-id`、`request-message`、`request-time`、`status`
- 发出（sent）：`request-id`、`sent-to-user-id`、`sent-message`、`sent-time`、`status`
- 好友（owned）：`friend-id`、`add-time`、`approve-reason`
  - 活跃标记：`status=active`
  - 删除标记：`status=ended`、`remove-time`、`remove-reason`

## 🔐 认证一致性

所有写操作通过认证中间件注入的 `AuthContext` 校验 `user_id` 一致性：

```
ensure_user_id_matches_token(req_user_id, &auth)?;
```

## 🔄 事务与并发

当前实现为简化的连接池读写；如需提升一致性，可将多步写入合并到事务中处理。删除为幂等操作，重复请求不会产生额外副作用。