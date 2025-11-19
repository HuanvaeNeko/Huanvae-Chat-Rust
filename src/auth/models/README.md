# Models 目录

数据模型层，定义数据结构和数据库映射。

## 📂 文件说明

### `user.rs` (112 行)
**用途**: 用户相关的数据模型

**包含的结构体**:

#### 1. `User` (数据库模型)
**用途**: 映射数据库 `users` 表

**字段**:
```rust
pub struct User {
    pub user_id: String,                              // 用户ID
    pub user_nickname: String,                        // 昵称
    pub user_password: String,                        // 密码哈希
    pub user_email: String,                           // 邮箱
    pub admin: String,                                // 是否管理员 ("true"/"false")
    pub user_owned_friends: String,                   // 好友列表
    pub user_pending_friend_requests: String,         // 待处理好友申请
    pub user_sent_friend_requests: String,            // 发出的好友申请
    pub user_joined_group_chats: String,              // 加入的群聊
    pub user_ai_conversation_data: String,            // AI对话数据
    pub user_chat_data_with_friends: String,          // 好友聊天数据
    pub user_file_data_and_url: String,               // 文件数据
    pub need_blacklist_check: bool,                   // 是否需要黑名单检查
    pub blacklist_check_expires_at: Option<NaiveDateTime>, // 黑名单检查过期时间
    pub created_at: NaiveDateTime,                    // 创建时间
    pub updated_at: NaiveDateTime,                    // 更新时间
}
```

**使用场景**:
- 从数据库查询用户信息时
- `sqlx::query_as::<_, User>()` 自动映射

**特殊注解**:
- `#[derive(FromRow)]` - SQLx 自动映射
- `#[serde(rename)]` - JSON 序列化时的字段名
- `#[sqlx(rename)]` - 数据库列名映射

---

#### 2. `RegisterRequest` (请求模型)
**用途**: 用户注册请求的数据结构

**字段**:
```rust
pub struct RegisterRequest {
    pub user_id: String,    // 用户提供的登录ID
    pub nickname: String,   // 昵称
    pub email: Option<String>,  // 邮箱（可选）
    pub password: String,   // 明文密码（传输中）
}
```

**调用时机**:
- `POST /api/auth/register` 请求体反序列化
- Axum 自动从 JSON 解析到此结构

**验证**:
- 在 `register_handler` 中被验证
- 使用 `utils/validator.rs` 进行格式检查

---

#### 3. `LoginRequest` (请求模型)
**用途**: 用户登录请求的数据结构

**字段**:
```rust
pub struct LoginRequest {
    pub user_id: String,               // 登录ID
    pub password: String,              // 明文密码
    pub device_info: Option<String>,   // 设备信息（可选）
    pub mac_address: Option<String>,   // MAC地址（可选）
}
```

**调用时机**:
- `POST /api/auth/login` 请求体反序列化

---

#### 4. `UserResponse` (响应模型)
**用途**: 返回给客户端的用户信息（不含敏感数据）

**字段**:
```rust
pub struct UserResponse {
    pub user_id: String,
    pub nickname: String,
    pub email: String,
    pub admin: bool,           // 转换为布尔值
    pub created_at: NaiveDateTime,
}
```

**特点**:
- **不包含密码**等敏感信息
- `admin` 字段从 `"true"/"false"` 字符串转换为布尔值

**调用时机**:
- 注册成功后返回
- 通过 `impl From<User> for UserResponse` 自动转换

---

### `claims.rs` (72 行)
**用途**: JWT Token 的 Claims 结构

**包含的结构体**:

#### 1. `AccessTokenClaims`
**用途**: Access Token（15分钟）的有效载荷

**字段**:
```rust
pub struct AccessTokenClaims {
    pub sub: String,         // Subject - 用户ID
    pub email: String,       // 用户邮箱
    pub device_id: String,   // 设备ID
    pub device_info: String, // 设备信息
    pub mac_address: String, // MAC地址
    pub jti: String,         // JWT ID - 唯一标识
    pub exp: i64,            // Expiration - 过期时间戳
    pub iat: i64,            // Issued At - 签发时间戳
}
```

**标准字段**:
- `sub` (Subject): 主题，通常是用户ID
- `jti` (JWT ID): JWT唯一标识，用于黑名单
- `exp` (Expiration): 过期时间
- `iat` (Issued At): 签发时间

**自定义字段**:
- `email`, `device_id`, `device_info`, `mac_address` - 业务字段

**调用时机**:
- 登录时生成 Access Token
- Token 验证时解析
- 中间件注入到请求扩展中

---

#### 2. `RefreshTokenClaims`
**用途**: Refresh Token（7天）的有效载荷

**字段**:
```rust
pub struct RefreshTokenClaims {
    pub sub: String,         // 用户ID
    pub device_id: String,   // 设备ID
    pub token_id: String,    // Token ID（对应数据库记录）
    pub exp: i64,            // 过期时间戳
    pub iat: i64,            // 签发时间戳
}
```

**特点**:
- 信息更少（安全考虑）
- `token_id` 对应数据库 `user-refresh-tokens` 表的 `token-id`
- 没有 `jti` 字段（使用 `token_id` 代替）

**调用时机**:
- 登录时生成 Refresh Token
- 刷新 Access Token 时验证和解析

---

### `refresh_token.rs` (69 行)
**用途**: Refresh Token 数据库模型

**包含的结构体**:

#### 1. `RefreshToken` (数据库模型)
**用途**: 映射数据库 `user-refresh-tokens` 表

**字段**:
```rust
pub struct RefreshToken {
    pub token_id: String,                    // Token ID
    pub user_id: String,                     // 用户ID
    pub refresh_token: String,               // JWT字符串
    pub device_id: String,                   // 设备ID
    pub device_info: Option<String>,         // 设备信息（JSON）
    pub ip_address: Option<String>,          // IP地址
    pub created_at: NaiveDateTime,           // 创建时间
    pub expires_at: NaiveDateTime,           // 过期时间
    pub last_used_at: Option<NaiveDateTime>, // 最后使用时间
    pub is_revoked: bool,                    // 是否已撤销
    pub revoked_at: Option<NaiveDateTime>,   // 撤销时间
    pub revoked_reason: Option<String>,      // 撤销原因
}
```

**使用场景**:
- 查询用户的所有 Refresh Token
- 验证 Refresh Token 是否被撤销
- 多设备登录管理

---

#### 2. `CreateRefreshToken` (参数模型)
**用途**: 创建新 Refresh Token 时的参数

**字段**:
```rust
pub struct CreateRefreshToken {
    pub token_id: String,
    pub user_id: String,
    pub refresh_token: String,
    pub device_id: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: NaiveDateTime,
}
```

**调用时机**:
- 登录时生成新 Token
- 在 `token_service.rs` 中构建并保存到数据库

---

### `device.rs` (39 行)
**用途**: 设备信息模型

**包含的结构体**:

#### 1. `Device` (响应模型)
**用途**: 返回给客户端的设备信息

**字段**:
```rust
pub struct Device {
    pub device_id: String,                   // 设备ID
    pub device_info: String,                 // 设备信息
    pub ip_address: Option<String>,          // IP地址
    pub last_used_at: Option<NaiveDateTime>, // 最后活跃时间
    pub created_at: NaiveDateTime,           // 首次登录时间
    pub is_current: bool,                    // 是否是当前设备
}
```

**使用场景**:
- 查看所有登录设备列表
- 显示设备详情

---

#### 2. `DeviceListResponse` (响应模型)
**用途**: 设备列表的响应结构

**字段**:
```rust
pub struct DeviceListResponse {
    pub devices: Vec<Device>,  // 设备列表
    pub total: usize,          // 总数
}
```

**调用时机**:
- `GET /api/auth/devices` 响应

---

#### 3. `RevokeDeviceRequest` (请求模型)
**用途**: 撤销设备请求（未使用）

**字段**:
```rust
pub struct RevokeDeviceRequest {
    pub device_id: String,
}
```

**注**: 当前实现从 URL 路径参数获取 `device_id`

---

### `mod.rs` (11 行)
**用途**: 模块导出

**导出内容**:
```rust
pub use claims::{AccessTokenClaims, RefreshTokenClaims};
pub use device::{Device, DeviceListResponse};
pub use refresh_token::{CreateRefreshToken, RefreshToken};
pub use user::{LoginRequest, RegisterRequest, User, UserResponse};
pub use token::{TokenResponse};
```

---

## 🔄 数据流转

### 注册流程
```
客户端 JSON
    ↓
RegisterRequest (反序列化)
    ↓
验证 + 密码哈希
    ↓
User (插入数据库)
    ↓
UserResponse (响应客户端)
```

### 登录流程
```
客户端 JSON
    ↓
LoginRequest (反序列化)
    ↓
User (从数据库查询)
    ↓
验证密码
    ↓
AccessTokenClaims + RefreshTokenClaims (生成JWT)
    ↓
RefreshToken (保存到数据库)
    ↓
TokenResponse (响应客户端)
```

### Token 验证流程
```
HTTP Header: Authorization: Bearer <token>
    ↓
解析 JWT
    ↓
AccessTokenClaims (验证并解析)
    ↓
注入到请求扩展
    ↓
Handler 使用 Claims
```

---

## 📊 字段映射规则

### Serde 重命名
用于 JSON 序列化/反序列化：
```rust
#[serde(rename = "user-id")]
pub user_id: String,
```

JSON中为 `"user-id"`，Rust中为 `user_id`

### SQLx 重命名
用于数据库列映射：
```rust
#[sqlx(rename = "user-id")]
pub user_id: String,
```

数据库列为 `"user-id"`，Rust中为 `user_id`

### 时间类型
- 数据库: `TIMESTAMP WITHOUT TIME ZONE`
- Rust: `chrono::NaiveDateTime`
- JSON: `"2025-11-17T02:41:07.933463"`

#### exp（Unix秒）到 NaiveDateTime 转换
- 旧用法（会产生废弃警告）：`NaiveDateTime::from_timestamp_opt(exp, 0)`
- 推荐用法：
  - `chrono::DateTime::from_timestamp(exp, 0).map(|dt| dt.naive_utc()).unwrap_or(Utc::now().naive_utc())`
- 说明：当 `exp` 非法时安全回退当前时间，避免写入无效过期时间；用于黑名单写入等场景与 Access Token 的 `exp` 对齐。

---

## 🎯 设计原则

**职责边界**:
- ✅ 定义数据结构
- ✅ 数据库字段映射
- ✅ JSON 序列化/反序列化
- ✅ 类型转换（如 `From<User> for UserResponse`）
- ❌ 不包含业务逻辑
- ❌ 不直接操作数据库

**安全性**:
- 响应模型不包含敏感信息（密码）
- JWT Claims 包含必要的验证信息
- Refresh Token 信息精简

**可维护性**:
- 清晰的命名约定
- 请求/响应/数据库模型分离
- 统一的字段命名映射

