# Models 目录

请求与响应数据模型，定义与序列化/反序列化结构。

## 📂 文件说明

### `request.rs`
用途: 提交/同意/拒绝好友申请的请求体结构。
包含:
- `SubmitFriendRequest`、`SubmitFriendResponse`
- `ApproveFriendRequest`
- `RejectFriendRequest`

### `list.rs`
用途: 列表查询响应结构。
包含:
- `SentRequestDto`、`PendingRequestDto`、`FriendDto`
- `ListResponse<T>` 泛型列表响应

## 🧩 约束与约定

- 所有 `user_id` 字段均与认证上下文一致
- 时间字段采用 RFC3339 字符串（与现有后端风格一致）
- 可选字段使用 `Option<String>` 表示