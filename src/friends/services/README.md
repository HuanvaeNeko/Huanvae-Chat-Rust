# Services 目录

业务服务层，封装好友申请、批准/拒绝、列表查询等核心逻辑与数据读写。

## 📂 文件说明

### `friends_service.rs`
用途: 提交/同意/拒绝好友申请的业务实现。

**错误类型**: 使用统一的 `crate::common::AppError`

**核心结构体**:
```rust
/// 好友服务（业务逻辑层）
pub struct FriendsService {
    pub db: PgPool,
}
```

**Handler 层状态**（在 `handlers/state.rs` 中定义）:
```rust
/// 好友模块 Handler 状态
pub struct FriendsState {
    pub service: FriendsService,  // 业务服务
    pub db: PgPool,
}
```

主要功能:
- `submit_request`：写入申请人的 `sent` 与目标用户的 `pending`；如检测到对向开放申请，调用 `auto_approve` 自动互通过
- `approve_request`：调用 `manual_approve` 标记 `sent/pending=approved` 并互相加入 `owned-friends`（新增好友记录显式 `status=active`）
- `reject_request`：将双方相关 `sent/pending` 标记为 `rejected`，可选记录拒绝原因
- `remove_friend`：删除好友（标记结束）。双向将 `user-owned-friends` 中匹配记录设置 `status=ended`，并记录 `remove-time/remove-reason`
- `verify_friendship`：验证两个用户之间是否存在活跃的好友关系（供其他模块调用）

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

```rust
fn ensure_user_id_matches_token(req_user_id: &str, auth: &AuthContext) -> Result<(), AppError> {
    if req_user_id != auth.user_id {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}
```

## ⚠️ 错误处理

所有方法返回 `Result<T, AppError>`：
- `AppError::BadRequest` - 请求参数错误或业务规则违反
- `AppError::Unauthorized` - 用户身份验证失败
- `AppError::Internal` - 数据库操作失败

## 🔄 事务与并发

使用数据库事务保证数据一致性：
- `approve_request` 和 `remove_friend` 使用事务处理双向好友关系
- 防止并发请求导致的数据不一致
- 删除为幂等操作，重复请求不会产生额外副作用