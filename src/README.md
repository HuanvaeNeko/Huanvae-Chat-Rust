# Source Code Structure

HuanVae Chat 源代码模块结构说明。

## 📂 模块列表

### 1. auth/ - 认证系统模块
用户认证、JWT Token 管理、设备管理、黑名单系统。

**详细文档**：[auth/README.md](./auth/README.md)

**主要功能**：
- 用户注册/登录
- 双 Token 机制（Access + Refresh）
- RSA 签名验证
- 多设备登录管理
- 智能黑名单检查

**API 端点**：
- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/devices`
- `DELETE /api/auth/devices/:id`

---

### 2. friends/ - 好友系统模块
好友关系管理，包括申请、接受、拒绝、删除等功能。

**详细文档**：[friends/README.md](./friends/README.md)

**主要功能**：
- 发送好友申请
- 接受/拒绝申请
- 查看好友列表
- 删除好友（软删除）

**API 端点**：
- `POST /api/friends/requests`
- `POST /api/friends/requests/approve`
- `POST /api/friends/requests/reject`
- `GET /api/friends/requests/sent`
- `GET /api/friends/requests/pending`
- `GET /api/friends`
- `POST /api/friends/remove`

---

### 3. profile/ - 个人资料模块
用户个人信息管理，包括查询、更新、密码修改、头像上传。

**详细文档**：[profile/README.md](./profile/README.md)

**主要功能**：
- 获取个人信息
- 更新邮箱和签名
- 修改密码
- 上传头像

**API 端点**：
- `GET /api/profile`
- `PUT /api/profile`
- `PUT /api/profile/password`
- `POST /api/profile/avatar`

---

### 4. storage/ - 对象存储模块
MinIO/S3 对象存储客户端封装，提供文件上传、下载、管理功能。

**详细文档**：[storage/README.md](./storage/README.md)

**主要功能**：
- S3/MinIO 客户端封装
- 头像上传服务
- 文件验证（类型、大小）
- Bucket 管理

**Bucket 分区**：
- `avatars/` - 用户头像（公开读取）
- `group-avatars/` - 群头像（公开读取）
- `group-messages/` - 群消息文件（需鉴权）
- `user-files/` - 用户文件（私有）

---

### 5. groups/ - 群聊系统模块
群聊管理，包括创建群聊、成员管理、角色管理、邀请码、入群机制等。

**详细文档**：[groups/README.md](./groups/README.md)

**主要功能**：
- 创建/解散群聊
- 成员管理（邀请、退出、移除）
- 角色管理（群主转让、管理员设置）
- 禁言功能
- 邀请码管理（直通码/普通码）
- 入群申请审核
- 群公告管理

**API 端点**：
- `POST /api/groups` - 创建群聊
- `GET /api/groups/my` - 我的群聊列表
- `POST /api/groups/:id/invite` - 邀请成员
- `POST /api/groups/:id/transfer` - 转让群主
- `POST /api/groups/:id/mute` - 禁言成员

---

### 6. group_messages/ - 群消息模块
群聊消息功能，支持文本、图片、视频、文件等消息类型。

**详细文档**：[group_messages/README.md](./group_messages/README.md)

**主要功能**：
- 发送群消息
- 获取群消息列表
- 删除消息（个人）
- 撤回消息（发送者2分钟内/管理员随时）

**API 端点**：
- `POST /api/group-messages` - 发送群消息
- `GET /api/group-messages` - 获取群消息
- `DELETE /api/group-messages/delete` - 删除消息
- `POST /api/group-messages/recall` - 撤回消息

---

### 7. websocket/ - WebSocket 实时通信模块
WebSocket 实时消息推送、未读消息通知、已读同步功能。

**详细文档**：[websocket/README.md](./websocket/README.md)

**主要功能**：
- WebSocket 实时连接
- 未读消息摘要推送
- 新消息实时通知
- 已读状态同步（可配置开关）
- 多设备消息同步

**端点**：
- `GET /ws?token=xxx` - WebSocket 连接
- `GET /ws/status` - 连接状态查询

**环境变量**：
- `WS_ENABLE_READ_RECEIPT` - 已读回执开关（true/false）
- `WS_HEARTBEAT_INTERVAL_SECONDS` - 心跳间隔
- `WS_CLIENT_TIMEOUT_SECONDS` - 客户端超时

---

### 8. turn/ - TURN 协调模块
分布式 TURN 服务器管理，提供 WebRTC ICE 配置服务。

**详细文档**：[turn/README.md](./turn/README.md)

**主要功能**：
- TURN 节点注册与管理
- 负载均衡节点选择
- 动态凭证签发（TURN REST API）
- 密钥自动轮换
- 实时心跳监控

**API 端点**：
- `GET /api/webrtc/ice-servers` - 获取 ICE 服务器配置（需认证）
- `WS /internal/turn-coordinator` - Agent WebSocket 连接（内部）

**环境变量**：
- `TURN_ENABLED` - 是否启用 TURN 功能（默认 false）
- `TURN_REALM` - TURN 域名
- `TURN_AGENT_AUTH_TOKEN` - Agent 认证令牌
- `TURN_CREDENTIAL_TTL_SECONDS` - 凭证有效期（默认 600 秒）
- `TURN_SECRET_ROTATION_HOURS` - 密钥轮换间隔（默认 24 小时）

---

### 9. webrtc_room/ - WebRTC 房间模块
WebRTC 实时音视频通信的房间管理和信令服务。

**详细文档**：[webrtc_room/README.md](./webrtc_room/README.md)

**主要功能**：
- 房间创建（登录用户）
- 房间加入（无需登录，密码验证）
- 信令 WebSocket（SDP/ICE Candidate 转发）
- TURN 服务器自动分配
- 临时凭证动态生成

**API 端点**：
- `POST /api/webrtc/rooms` - 创建房间（需登录）
- `POST /api/webrtc/rooms/{room_id}/join` - 加入房间（无需登录）
- `WS /ws/webrtc/rooms/{room_id}?token=xxx` - 信令 WebSocket

**使用流程**：
1. 登录用户创建房间，获得 `room_id` 和 `password`
2. 分享房间号和密码给朋友
3. 参与者使用房间号+密码加入，获得 `ws_token` 和 `ice_servers`
4. 所有人连接信令 WebSocket，交换 SDP 和 ICE Candidate
5. 建立 P2P 连接，开始音视频通话

---

### 10. common/ - 公共模块
统一错误类型和 API 响应格式，供所有模块使用。

**主要功能**：
- `AppError` - 统一错误类型，自动转换为 HTTP 响应
- `ApiResponse<T>` - 统一 API 响应格式

**使用示例**：
```rust
use huanvae_chat::{AppError, ApiResponse};

// Handler 返回统一响应
pub async fn handler() -> Result<Json<ApiResponse<Data>>, AppError> {
    let data = service.get_data().await?;
    Ok(Json(ApiResponse::success(data)))
}
```

---

### 11. config.rs - 配置管理模块
从环境变量加载所有配置项，将硬编码的时间常量等提取为可配置参数。

**主要配置项**：
- `TokenConfig` - Token 相关配置（有效期、黑名单检查窗口）
- `StorageConfig` - 存储相关配置（预签名 URL 有效期）
- `MessageConfig` - 消息相关配置（撤回时间窗口）
- `WebSocketConfig` - WebSocket 相关配置（已读回执开关、心跳间隔）

**环境变量**：
- `ACCESS_TOKEN_TTL_SECONDS` - Access Token 有效期（默认 900 秒）
- `REFRESH_TOKEN_TTL_SECONDS` - Refresh Token 有效期（默认 604800 秒）
- `BLACKLIST_CHECK_WINDOW_SECONDS` - 黑名单检查窗口（默认 900 秒）
- `MESSAGE_RECALL_WINDOW_SECONDS` - 消息撤回时限（默认 120 秒）
- `WS_ENABLE_READ_RECEIPT` - 已读回执功能开关（默认 true）
- `WS_HEARTBEAT_INTERVAL_SECONDS` - 心跳间隔（默认 30 秒）
- `WS_CLIENT_TIMEOUT_SECONDS` - 客户端超时（默认 60 秒）

---

### 12. app_state.rs - 应用状态管理
统一管理所有服务实例，避免在 main.rs 中创建大量分散的 State 对象。

**主要功能**：
- 集中创建和管理所有服务实例
- 提供便捷方法生成各模块所需的 State
- 简化 main.rs 中的初始化代码
- 管理 WebSocket 连接管理器和通知服务

---

## 🏗️ 模块设计原则

所有模块遵循统一的设计原则：

### 1. 三层架构
```
handlers/    → HTTP 请求处理层（路由、参数提取、响应构建）
services/    → 业务逻辑层（核心业务处理、数据库操作）
models/      → 数据模型层（请求/响应结构定义）
```

### 2. 职责清晰
- **Handlers**：仅处理 HTTP 相关逻辑，不包含业务逻辑
- **Services**：专注业务逻辑，不关心 HTTP 细节
- **Models**：纯数据结构，包含验证规则

### 3. 错误处理
- 使用统一的 `AppError` 错误类型（位于 `common/errors.rs`）
- 使用统一的 `ApiResponse<T>` 响应格式（位于 `common/response.rs`）
- Handler 层直接返回 `Result<ApiResponse<T>, AppError>`
- 详细的错误日志记录（内部错误不暴露给用户）

### 4. 异步优先
- 所有 I/O 操作使用 `async/await`
- 数据库操作使用 sqlx 异步客户端
- HTTP 框架基于 Axum (tokio)

### 5. 类型安全
- 充分利用 Rust 类型系统
- 使用 `serde` 进行序列化/反序列化
- 使用 `validator` 进行数据验证

---

## 📖 开发指南

### 添加新模块

1. **创建目录结构**：
```bash
src/new_module/
├── handlers/
│   ├── mod.rs
│   └── routes.rs
├── models/
│   ├── mod.rs
│   ├── request.rs
│   └── response.rs
├── services/
│   ├── mod.rs
│   └── service_name.rs
├── mod.rs
└── README.md
```

2. **在 lib.rs 中导出**：
```rust
pub mod new_module;
```

3. **在 main.rs 中注册路由**：
```rust
use huanvae_chat::new_module::handlers::routes::module_routes;

let app = Router::new()
    .merge(module_routes(/* 参数 */));
```

4. **编写 README**：
参考现有模块的 README 格式，包含：
- 模块概述
- 目录结构
- 路由映射
- 数据模型
- 使用示例

---

## 🔗 模块依赖关系

```
main.rs
  ├─ app_state  → 统一状态管理
  ├─ config     → 配置管理
  ├─ common     → 公共类型（AppError, ApiResponse）
  ├─ auth       → 认证系统
  ├─ friends    → 好友系统（依赖 auth middleware）
  ├─ profile    → 个人资料（依赖 auth middleware + storage）
  ├─ storage    → 对象存储
  ├─ friends_messages → 好友消息（依赖 auth middleware）
  ├─ groups     → 群聊系统（依赖 auth middleware）
  ├─ group_messages → 群消息（依赖 auth middleware + groups）
  ├─ websocket  → WebSocket 实时通信（依赖 auth + friends_messages + group_messages）
  ├─ turn       → TURN 协调服务（依赖 auth middleware）
  └─ webrtc_room → WebRTC 房间服务（依赖 auth middleware + turn）
```

**依赖说明**：
- `common` 提供统一错误类型和响应格式，被所有模块使用
- `config` 提供可配置的时间常量，被需要时间参数的模块使用
- `app_state` 集中管理所有服务实例
- `auth` 提供认证中间件和用户上下文
- `friends`、`profile`、`friends_messages` 使用 `auth_guard` 保护路由
- `profile` 使用 `storage` 上传头像
- `storage` 可被任何需要文件存储的模块使用
- `groups` 提供群聊管理功能，使用 `auth_guard` 保护路由
- `group_messages` 依赖 `groups` 模块的成员验证服务
- `websocket` 提供实时消息推送，集成 `friends_messages` 和 `group_messages` 的通知

---

## 📚 文档索引

### 模块文档
- [auth/README.md](./auth/README.md) - 认证系统
- [friends/README.md](./friends/README.md) - 好友系统
- [profile/README.md](./profile/README.md) - 个人资料
- [storage/README.md](./storage/README.md) - 对象存储
- [groups/README.md](./groups/README.md) - 群聊系统
- [group_messages/README.md](./group_messages/README.md) - 群消息
- [websocket/README.md](./websocket/README.md) - WebSocket 实时通信
- [turn/README.md](./turn/README.md) - TURN 协调服务
- [webrtc_room/README.md](./webrtc_room/README.md) - WebRTC 房间服务
- [common/README.md](./common/README.md) - 公共模块

### 子模块文档
每个模块的子目录也有详细文档：
- `handlers/README.md` - 请求处理器说明
- `models/README.md` - 数据模型说明
- `services/README.md` - 业务服务说明

### API 文档
前端调用示例（Fetch API）：
- [接口调取文档/auth/](../接口调取文档/auth/)
- [接口调取文档/friends/](../接口调取文档/friends/)
- [接口调取文档/profile/](../接口调取文档/profile/)

---

## 🎯 编码规范

### 命名约定
- **文件名**：蛇形命名（`snake_case.rs`）
- **结构体/枚举**：大驼峰（`PascalCase`）
- **函数/变量**：蛇形命名（`snake_case`）
- **常量**：大写蛇形（`UPPER_SNAKE_CASE`）

### 注释规范
```rust
/// 函数文档注释（三斜杠）
/// 
/// # 参数
/// - `param1`: 参数说明
/// 
/// # 返回
/// - `Ok(result)`: 成功情况
/// - `Err(error)`: 错误情况
pub async fn function_name(param1: Type) -> Result<Type, Error> {
    // 实现逻辑
}
```

### 错误处理
```rust
// 好的做法：使用 ? 操作符
let result = operation().await?;

// 记录错误日志
match operation().await {
    Ok(result) => { /* 处理成功 */ },
    Err(e) => {
        error!("Operation failed: {}", e);
        return Err(e.into());
    }
}
```

---

## 🚀 快速开始

1. **阅读模块 README**：了解模块功能和 API
2. **查看 handlers/**：理解 HTTP 接口定义
3. **查看 models/**：了解请求/响应结构
4. **查看 services/**：理解业务逻辑实现
5. **参考接口文档**：查看前端调用示例

---

## 👥 贡献指南

1. 遵循现有模块的结构和命名规范
2. 为新功能编写完整的 README
3. 添加适当的错误处理和日志
4. 确保代码通过 `cargo clippy` 检查
5. 编写前端调用示例文档

