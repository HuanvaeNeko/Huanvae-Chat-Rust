# Middleware 目录

中间件层，负责请求的预处理和拦截。

## 📂 文件说明

### `auth_guard.rs` (116 行)
**用途**: JWT 认证中间件

**主要功能**:
1. **Token 提取**: 从 HTTP 请求头 `Authorization: Bearer <token>` 中提取 JWT
2. **Token 验证**: 
   - 验证 JWT 签名（使用 RSA 公钥）
   - 检查 Token 是否过期
   - 解析 Token Claims
3. **黑名单检查**:
   - 查询用户的 `need-blacklist-check` 标志
   - 如果启用（安全事件后15分钟内），检查 Token 是否在黑名单中
   - 如果未启用，跳过黑名单查询（性能优化）
4. **用户信息注入**: 将 JWT Claims 注入到请求扩展中，供后续 handler 使用

**调用时机**:
- **仅在需要认证的路由上使用**
- 在 `routes.rs` 中通过 `.layer(middleware::from_fn_with_state(...))` 应用
- 在 handler 执行**之前**被调用

**应用的路由**:
```
POST   /api/auth/logout         ✅ 需要认证
GET    /api/auth/devices        ✅ 需要认证
DELETE /api/auth/devices/{id}   ✅ 需要认证

POST   /api/auth/register       ❌ 公开
POST   /api/auth/login          ❌ 公开
POST   /api/auth/refresh        ❌ 公开
```

**工作流程**:
```
1. 接收 HTTP 请求
     ↓
2. 检查 Authorization 头
     ↓ (不存在)
   返回 401 Unauthorized
     ↓ (存在)
3. 提取并验证 JWT
     ↓ (无效)
   返回 401 Invalid Token
     ↓ (有效)
4. 查询用户的黑名单检查状态
     ↓
5a. need-blacklist-check = false
    → 跳过黑名单查询 (高性能路径)
    → 直接通过
     ↓
5b. need-blacklist-check = true
    → 查询黑名单数据库
    → 如果在黑名单中，返回 401
    → 如果不在，通过
     ↓
6. 将 Claims 注入请求扩展
     ↓
7. 继续执行后续 handler
```

**智能黑名单检查**:

这是一个性能优化设计：

- **正常情况** (99%的情况):
  - `need-blacklist-check = false`
  - **跳过**黑名单数据库查询
  - 每个请求节省 ~2ms 查询时间

- **安全事件后** (用户修改密码、远程登出):
  - 设置 `need-blacklist-check = true`
  - 设置 `blacklist-check-expires-at = now + 15分钟`
  - **启用**黑名单检查，确保旧 Token 被拒绝
  - 15分钟后自动恢复为 `false`（定时任务清理）

**依赖**:
- `services/token_service.rs` - Token 验证
- `services/blacklist_service.rs` - 黑名单查询
- `models/claims.rs` - JWT Claims 结构

**State 结构**:
```rust
pub struct AuthState {
    pub key_manager: Arc<KeyManager>,  // RSA密钥管理器
    pub db: PgPool,                    // 数据库连接池
}
```

**注入的扩展**:
```rust
pub struct AccessTokenClaims {
    pub sub: String,         // 用户ID
    pub email: String,       // 邮箱
    pub device_id: String,   // 设备ID
    pub device_info: String, // 设备信息
    pub mac_address: String, // MAC地址
    pub jti: String,         // JWT唯一标识
    pub exp: i64,            // 过期时间
    pub iat: i64,            // 签发时间
}
```

**Handler 中使用**:
```rust
pub async fn logout_handler(
    Extension(claims): Extension<AccessTokenClaims>,  // 自动注入
    State(state): State<LogoutState>,
) -> Result<Json<LogoutResponse>, AuthError> {
    // 使用 claims.sub (用户ID)
    // 使用 claims.device_id (设备ID)
    // ...
}
```

**错误响应**:
- `401 Unauthorized` - Token 缺失或无效
- `401 Invalid Token` - Token 格式错误或签名验证失败
- `401 Token已被撤销` - Token 在黑名单中
- `401 Token已过期` - Token 超过有效期

---

### `mod.rs` (5 行)
**用途**: 模块导出

**主要功能**:
- 声明 `auth_guard` 子模块
- 重新导出 `auth_guard` 函数和 `AuthState` 结构体

**导出内容**:
```rust
pub use auth_guard::{auth_guard, AuthState};
```

---

## 🔒 安全特性

### 1. RSA 签名验证
- 使用 2048位 RSA 密钥对
- 私钥签名，公钥验证
- 防止 Token 伪造

### 2. 智能黑名单
- 正常情况下跳过黑名单查询（高性能）
- 安全事件后启用15分钟黑名单检查
- 自动过期恢复

### 3. 过期时间检查
- Access Token: 15分钟有效期
- 自动拒绝过期 Token

### 4. JWT 唯一标识 (jti)
- 每个 Token 都有唯一的 `jti` 字段
- 用于黑名单精确匹配

---

## 🔧 配置示例

### 在 routes.rs 中应用中间件

```rust
use axum::{middleware, Router};
use crate::auth::middleware::{auth_guard, AuthState};

// 创建认证状态
let auth_state = AuthState {
    key_manager: Arc::new(key_manager),
    db: db_pool.clone(),
};

// 需要认证的路由
let protected_routes = Router::new()
    .route("/logout", post(logout_handler))
    .route("/devices", get(list_devices_handler))
    .route("/devices/{id}", delete(revoke_device_handler))
    .layer(middleware::from_fn_with_state(auth_state, auth_guard));  // 应用中间件

// 公开路由（不需要中间件）
let public_routes = Router::new()
    .route("/register", post(register_handler))
    .route("/login", post(login_handler))
    .route("/refresh", post(refresh_token_handler));

// 合并路由
Router::new()
    .merge(public_routes)
    .merge(protected_routes)
```

---

## 📊 性能优化

### 智能黑名单检查流程

```
用户登录
  ↓
need-blacklist-check = false (默认)
  ↓
后续所有请求：
  - 验证 JWT 签名 ✓
  - 检查过期时间 ✓
  - 跳过黑名单查询 ✓ (节省 ~2ms)
  ↓
[用户修改密码 / 远程登出]
  ↓
need-blacklist-check = true (15分钟)
blacklist-check-expires-at = now + 15min
  ↓
后续所有请求（15分钟内）：
  - 验证 JWT 签名 ✓
  - 检查过期时间 ✓
  - 查询黑名单数据库 ✓
  ↓
15分钟后（定时任务）
  ↓
need-blacklist-check = false (自动恢复)
  ↓
后续请求恢复高性能模式
```

---

## 🎯 设计理念

**职责边界**:
- ✅ 认证和授权检查
- ✅ Token 验证和解析
- ✅ 黑名单检查
- ✅ 用户信息注入
- ❌ 不处理业务逻辑
- ❌ 不直接修改数据

**性能优先**:
- 智能黑名单检查（99%情况跳过）
- 快速失败（Token 无效立即返回）
- 异步数据库查询

**安全优先**:
- RSA 签名验证
- 黑名单机制
- 过期时间检查
- 详细的错误日志

