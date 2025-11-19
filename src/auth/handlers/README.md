# Handlers 目录

HTTP 请求处理器层，负责接收和响应 HTTP 请求。

## 📂 文件说明

### `register.rs` (84 行)
**用途**: 处理用户注册请求

**主要功能**:
- 验证用户输入（邮箱、昵称、密码强度）
- 检查用户ID和邮箱是否已存在
- 使用 bcrypt 加密密码
- 创建新用户并写入数据库
- 返回用户信息（不含密码）

**调用时机**: 
- 当客户端发送 `POST /api/auth/register` 请求时
- 无需认证，公开端点

**依赖**:
- `utils/validator.rs` - 输入验证
- `utils/password.rs` - 密码哈希
- `models/user.rs` - 用户模型

**返回**: `UserResponse` (JSON)

---

### `login.rs` (55 行)
**用途**: 处理用户登录请求

**主要功能**:
- 验证用户ID和密码
- 生成 Access Token（15分钟）和 Refresh Token（7天）
- 记录设备信息到数据库
- 返回双Token响应

**调用时机**:
- 当客户端发送 `POST /api/auth/login` 请求时
- 无需认证，公开端点

**依赖**:
- `services/token_service.rs` - Token生成
- `utils/password.rs` - 密码验证
- `models/user.rs` - 用户查询

**返回**: `TokenResponse` (包含 access_token 和 refresh_token)

---

### `refresh_token.rs` (36 行)
**用途**: 刷新 Access Token

**主要功能**:
- 验证 Refresh Token 的有效性
- 检查 Token 是否被撤销
- 生成新的 Access Token
- 更新 Refresh Token 的最后使用时间

**调用时机**:
- 当客户端的 Access Token 即将过期或已过期时
- 客户端发送 `POST /api/auth/refresh` 请求
- 无需认证，公开端点（但需要有效的 Refresh Token）

**依赖**:
- `services/token_service.rs` - Token刷新逻辑

**返回**: `TokenResponse` (新的 access_token)

---

### `logout.rs` (76 行)
**用途**: 处理用户登出请求

**主要功能**:
- 从 JWT Claims 中提取用户和设备信息
- 撤销当前设备的 Refresh Token
- 将当前 Access Token 加入黑名单
- 启用用户的黑名单检查（15分钟）

**调用时机**:
- 当用户主动登出时
- 客户端发送 `POST /api/auth/logout` 请求
- **需要认证**，必须携带有效的 Access Token

**依赖**:
- `middleware/auth_guard.rs` - 认证检查
- `services/token_service.rs` - Token查询
- `services/blacklist_service.rs` - 黑名单管理

**返回**: 成功消息 (JSON)

---

### `revoke_device.rs` (76 行)
**用途**: 设备管理（查看和撤销）

**主要功能**:
1. **`list_devices_handler`**:
   - 查询用户的所有登录设备
   - 标记当前设备
   - 显示设备详细信息（设备ID、信息、IP、最后活跃时间）

2. **`revoke_device_handler`**:
   - 远程撤销指定设备的访问权限
   - 删除该设备的 Refresh Token

**调用时机**:
- `GET /api/auth/devices` - 查看所有设备
- `DELETE /api/auth/devices/{device_id}` - 撤销指定设备
- **需要认证**，必须携带有效的 Access Token

**依赖**:
- `services/device_service.rs` - 设备管理逻辑

**返回**: 
- GET: `DeviceListResponse` (设备列表)
- DELETE: 成功消息

---

### `routes.rs` (47 行)
**用途**: 路由配置和组装

**主要功能**:
- 定义所有认证相关的 API 路由
- 区分公开路由和需要认证的路由
- 配置路由的状态（State）和中间件

**路由映射**:
```
公开路由:
  POST /api/auth/register  → register_handler
  POST /api/auth/login     → login_handler
  POST /api/auth/refresh   → refresh_token_handler

需要认证的路由:
  POST   /api/auth/logout         → logout_handler
  GET    /api/auth/devices        → list_devices_handler
  DELETE /api/auth/devices/{id}   → revoke_device_handler
```

**调用时机**:
- 应用启动时，由 `main.rs` 调用 `create_auth_routes()` 函数
- 构建完整的路由树

---

### `mod.rs` (15 行)
**用途**: 模块导出

**主要功能**:
- 声明所有 handler 子模块
- 重新导出关键结构体和函数，供外部使用

**导出内容**:
- `create_auth_routes` - 路由创建函数
- `RegisterState`, `LoginState`, `RefreshTokenState`, `LogoutState`, `DeviceState` - 各个处理器的状态结构体

---

## 🔄 调用流程

```
客户端请求
    ↓
路由匹配 (routes.rs)
    ↓
中间件检查 (auth_guard.rs) [可选，仅需认证的端点]
    ↓
Handler 处理 (register/login/logout/etc.)
    ↓
调用 Services 层
    ↓
操作 Models 和数据库
    ↓
返回 HTTP 响应
```

## 📝 使用示例

### 注册新用户
```bash
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "nickname": "张三",
    "email": "zhangsan@example.com",
    "password": "SecurePass123"
  }'
```

### 用户登录
```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "password": "SecurePass123",
    "device_info": "Chrome on Windows"
  }'
```

### 刷新 Token
```bash
curl -X POST http://localhost:8080/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "eyJ..."}'
```

### 查看设备列表
```bash
curl -X GET http://localhost:8080/api/auth/devices \
  -H "Authorization: Bearer <access_token>"
```

### 登出
```bash
curl -X POST http://localhost:8080/api/auth/logout \
  -H "Authorization: Bearer <access_token>"
```

---

## 🏗️ 架构设计

**职责边界**:
- ✅ 接收 HTTP 请求
- ✅ 参数验证和反序列化
- ✅ 调用 Services 层处理业务逻辑
- ✅ 序列化响应并返回
- ❌ 不直接操作数据库（通过 Services）
- ❌ 不包含复杂业务逻辑

**状态管理**:
每个 handler 都有自己的 State 结构体，包含：
- 数据库连接池 (`PgPool`)
- 相关的 Service 实例（如 `TokenService`）

**错误处理**:
所有 handler 返回 `Result<Json<T>, AuthError>`，错误会被自动转换为 HTTP 错误响应。

