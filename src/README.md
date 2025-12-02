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
- `group-files/` - 群文件（私有，待实现）
- `user-files/` - 用户文件（私有，待实现）

---

### 5. common/ - 公共模块
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

### 6. config.rs - 配置管理模块
从环境变量加载所有配置项，将硬编码的时间常量等提取为可配置参数。

**主要配置项**：
- `TokenConfig` - Token 相关配置（有效期、黑名单检查窗口）
- `StorageConfig` - 存储相关配置（预签名 URL 有效期）
- `MessageConfig` - 消息相关配置（撤回时间窗口）

**环境变量**：
- `ACCESS_TOKEN_TTL_SECONDS` - Access Token 有效期（默认 900 秒）
- `REFRESH_TOKEN_TTL_SECONDS` - Refresh Token 有效期（默认 604800 秒）
- `BLACKLIST_CHECK_WINDOW_SECONDS` - 黑名单检查窗口（默认 900 秒）
- `MESSAGE_RECALL_WINDOW_SECONDS` - 消息撤回时限（默认 120 秒）

---

### 7. app_state.rs - 应用状态管理
统一管理所有服务实例，避免在 main.rs 中创建大量分散的 State 对象。

**主要功能**：
- 集中创建和管理所有服务实例
- 提供便捷方法生成各模块所需的 State
- 简化 main.rs 中的初始化代码

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
  └─ friends_messages → 好友消息（依赖 auth middleware）
```

**依赖说明**：
- `common` 提供统一错误类型和响应格式，被所有模块使用
- `config` 提供可配置的时间常量，被需要时间参数的模块使用
- `app_state` 集中管理所有服务实例
- `auth` 提供认证中间件和用户上下文
- `friends`、`profile`、`friends_messages` 使用 `auth_guard` 保护路由
- `profile` 使用 `storage` 上传头像
- `storage` 可被任何需要文件存储的模块使用

---

## 📚 文档索引

### 模块文档
- [auth/README.md](./auth/README.md) - 认证系统
- [friends/README.md](./friends/README.md) - 好友系统
- [profile/README.md](./profile/README.md) - 个人资料
- [storage/README.md](./storage/README.md) - 对象存储
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

