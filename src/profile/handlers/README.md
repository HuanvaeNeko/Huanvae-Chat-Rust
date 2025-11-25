# Profile Handlers

HTTP 请求处理器层，负责接收客户端请求、验证数据、调用服务层、返回响应。

## 📁 文件列表

- `get_profile.rs` - 获取个人信息
- `update_profile.rs` - 更新个人信息（邮箱/签名）
- `update_password.rs` - 修改密码
- `upload_avatar.rs` - 上传头像
- `routes.rs` - 路由配置和状态管理
- `mod.rs` - Handler 导出

## 🔗 路由定义 (`routes.rs`)

### ProfileAppState

共享应用状态：

```rust
#[derive(Clone)]
pub struct ProfileAppState {
    pub pool: PgPool,              // 数据库连接池
    pub s3_client: Arc<S3Client>,  // S3 客户端
}
```

### profile_routes()

配置所有 profile 相关路由：

```rust
pub fn profile_routes(pool: PgPool, s3_client: Arc<S3Client>, auth_state: AuthState) -> Router
```

**路由映射**：
- `GET /api/profile` → `get_profile`
- `PUT /api/profile` → `update_profile`
- `PUT /api/profile/password` → `update_password`
- `POST /api/profile/avatar` → `upload_avatar`

**中间件**：所有路由通过 `auth_guard` 保护。

## 📄 Handler 详解

### get_profile.rs

**功能**：获取当前用户的完整个人信息（不含密码）。

**端点**：`GET /api/profile`

**鉴权**：Bearer Token（必需）

**请求参数**：无

**响应**：
```json
{
  "data": {
    "user_id": "testuser001",
    "user_nickname": "测试用户",
    "user_email": "test@example.com",
    "user_signature": "Hello, world!",
    "user_avatar_url": "http://localhost:9000/avatars/testuser001.jpg",
    "admin": "false",
    "created_at": "2025-11-25T11:00:02.221791Z",
    "updated_at": "2025-11-25T11:03:40.730806Z"
  }
}
```

**实现要点**：
- 从 `AuthContext` 提取 `user_id`
- 调用 `ProfileService::get_profile()`
- 返回 JSON 格式的用户信息

### update_profile.rs

**功能**：更新用户的邮箱和/或个性签名。

**端点**：`PUT /api/profile`

**鉴权**：Bearer Token（必需）

**请求体**：
```json
{
  "email": "new@example.com",      // 可选
  "signature": "My new signature"  // 可选
}
```

**响应**：
```json
{
  "message": "Profile updated successfully"
}
```

**验证规则**：
- `email`: 必须是有效邮箱格式
- `signature`: 最长 200 字符

**实现要点**：
- 使用 `validator::Validate` 验证请求
- 至少提供一个字段（email 或 signature）
- 动态构建 SQL UPDATE 语句

### update_password.rs

**功能**：修改用户密码。

**端点**：`PUT /api/profile/password`

**鉴权**：Bearer Token（必需）

**请求体**：
```json
{
  "old_password": "oldpass123",
  "new_password": "newpass123"
}
```

**响应**：
```json
{
  "message": "Password updated successfully"
}
```

**错误响应**：
```json
{
  "error": "Old password is incorrect"
}
```

**验证规则**：
- `old_password`: 至少 6 字符
- `new_password`: 6-100 字符

**实现要点**：
- 先验证旧密码是否正确
- 使用 bcrypt 验证和加密
- 返回 401 状态码如果旧密码错误

**安全特性**：
- 旧密码验证确保是本人操作
- 新密码立即生效
- 密码哈希使用 bcrypt

### upload_avatar.rs

**功能**：上传用户头像到 MinIO。

**端点**：`POST /api/profile/avatar`

**鉴权**：Bearer Token（必需）

**请求格式**：`multipart/form-data`

**表单字段**：
- `avatar` 或 `file`: 图片文件

**响应**：
```json
{
  "avatar_url": "http://localhost:9000/avatars/user-123.jpg",
  "message": "Avatar uploaded successfully"
}
```

**支持格式**：jpg, jpeg, png, gif, webp

**大小限制**：最大 5MB

**实现要点**：
1. 使用 `Multipart` 提取器读取文件
2. 查找 `avatar` 或 `file` 字段
3. 调用 `AvatarService::upload_avatar()` 验证并上传
4. 调用 `ProfileService::update_avatar_url()` 更新数据库
5. 返回公开访问 URL

**错误处理**：
```json
{
  "error": "No file uploaded. Use field name 'avatar' or 'file'"
}
```

```json
{
  "error": "File too large. Maximum size: 5 MB, got: 8.42 MB"
}
```

```json
{
  "error": "Unsupported file format. Allowed: jpg, jpeg, png, gif, webp"
}
```

## 🔄 请求流程

### 典型请求流程

```
客户端请求
    ↓
Axum Router
    ↓
auth_guard 中间件
    ├─ 验证 Token
    ├─ 提取 user_id
    └─ 注入 AuthContext
    ↓
Handler 函数
    ├─ 提取请求参数
    ├─ 验证数据格式
    ├─ 调用 Service 层
    └─ 构建 JSON 响应
    ↓
返回给客户端
```

## 🎯 实现规范

### Handler 函数签名

```rust
pub async fn handler_name(
    State(state): State<ProfileAppState>,      // 应用状态
    Extension(auth_ctx): Extension<AuthContext>, // 认证上下文
    /* 其他提取器 */
) -> impl IntoResponse {
    // 处理逻辑
}
```

### 错误处理

使用元组返回：

```rust
match service.operation().await {
    Ok(result) => (
        StatusCode::OK,
        Json(json!({ "data": result }))
    ),
    Err(e) => (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": e.to_string() }))
    ),
}
```

### 日志记录

使用 `tracing` 宏：

```rust
use tracing::{info, error};

info!("Uploading avatar for user: {}", user_id);
error!("Failed to upload avatar: {}", e);
```

## 🔒 安全最佳实践

1. **始终验证用户身份**
   ```rust
   Extension(auth_ctx): Extension<AuthContext>
   let user_id = &auth_ctx.user_id;
   ```

2. **验证输入数据**
   ```rust
   if let Err(e) = request.validate() {
       return (StatusCode::BAD_REQUEST, Json(json!({ "error": e })));
   }
   ```

3. **不在响应中暴露敏感信息**
   - 不返回密码
   - 错误信息不包含内部实现细节

4. **使用适当的 HTTP 状态码**
   - 200: 成功
   - 400: 请求参数错误
   - 401: 认证失败
   - 500: 服务器内部错误

## 📝 添加新 Handler

1. **创建文件**：`src/profile/handlers/new_feature.rs`

2. **实现函数**：
```rust
use crate::auth::middleware::AuthContext;
use crate::profile::handlers::routes::ProfileAppState;
use axum::{extract::State, http::StatusCode, Extension, Json};

pub async fn new_feature(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    // 实现逻辑
    (StatusCode::OK, Json(json!({ "message": "Success" })))
}
```

3. **导出 Handler**：在 `mod.rs` 中添加：
```rust
pub mod new_feature;
pub use new_feature::new_feature;
```

4. **注册路由**：在 `routes.rs` 中添加：
```rust
Router::new()
    .route("/api/profile/new-feature", post(new_feature))
```

## 🧪 测试建议

使用 `curl` 测试：

```bash
# 获取个人信息
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/api/profile

# 更新信息
curl -X PUT \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"email":"new@example.com"}' \
     http://localhost:8080/api/profile

# 上传头像
curl -X POST \
     -H "Authorization: Bearer $TOKEN" \
     -F "avatar=@avatar.jpg" \
     http://localhost:8080/api/profile/avatar
```

