# Profile 模块

用户个人资料管理模块，提供个人信息查询、更新（邮箱、签名）、密码修改、头像上传等功能。

## 📂 目录结构

```
src/profile/
  ├─ handlers/         // HTTP 请求处理器层
  │   ├─ get_profile.rs       // 获取个人信息
  │   ├─ update_profile.rs    // 更新个人信息
  │   ├─ update_password.rs   // 修改密码
  │   ├─ upload_avatar.rs     // 上传头像
  │   ├─ routes.rs            // 路由配置
  │   └─ mod.rs               // Handler 导出
  ├─ models/           // 请求/响应数据模型
  │   ├─ request.rs           // 请求体模型
  │   ├─ response.rs          // 响应体模型
  │   └─ mod.rs               // Model 导出
  ├─ services/         // 业务逻辑与数据库操作
  │   ├─ profile_service.rs   // 个人资料服务
  │   └─ mod.rs               // Service 导出
  └─ mod.rs            // 模块导出
```

## 🔗 路由映射

| 方法 | 路径 | 功能 | 鉴权 |
|------|------|------|------|
| GET | `/api/profile` | 获取当前用户信息 | ✅ Bearer Token |
| PUT | `/api/profile` | 更新邮箱/签名 | ✅ Bearer Token |
| PUT | `/api/profile/password` | 修改密码 | ✅ Bearer Token |
| POST | `/api/profile/avatar` | 上传头像 | ✅ Bearer Token |

**所有接口**均通过 `auth_guard` 中间件保护，使用 `Extension<AuthContext>` 获取当前用户身份。

## 📊 数据模型

### 请求模型

#### UpdateProfileRequest
```rust
pub struct UpdateProfileRequest {
    pub email: Option<String>,        // 新邮箱（可选）
    pub signature: Option<String>,    // 个性签名（可选）
}
```

**验证规则**：
- `email`: 必须是有效的邮箱格式
- `signature`: 最长 200 字符

#### UpdatePasswordRequest
```rust
pub struct UpdatePasswordRequest {
    pub old_password: String,         // 旧密码
    pub new_password: String,         // 新密码
}
```

**验证规则**：
- `old_password`: 至少 6 字符
- `new_password`: 6-100 字符

### 响应模型

#### ProfileResponse
```rust
pub struct ProfileResponse {
    pub user_id: String,              // 用户 ID
    pub user_nickname: String,        // 昵称
    pub user_email: Option<String>,   // 邮箱
    pub user_signature: Option<String>, // 个性签名
    pub user_avatar_url: Option<String>, // 头像 URL
    pub admin: String,                // 是否管理员
    pub created_at: DateTime<Utc>,    // 创建时间
    pub updated_at: DateTime<Utc>,    // 更新时间
}
```

**注意**：不包含密码字段（安全考虑）。

#### AvatarUploadResponse
```rust
pub struct AvatarUploadResponse {
    pub avatar_url: String,           // 头像 URL
    pub message: String,              // 成功消息
}
```

## 🔄 业务流程

### 1. 获取个人信息

```
客户端 → GET /api/profile + Bearer Token
       → auth_guard 验证 Token
       → 提取 user_id
       → 查询数据库
       → 返回用户信息（不含密码）
```

### 2. 更新个人信息

```
客户端 → PUT /api/profile + { email?, signature? }
       → auth_guard 验证
       → validator 验证数据格式
       → 更新数据库（仅更新提供的字段）
       → 返回成功消息
```

### 3. 修改密码

```
客户端 → PUT /api/profile/password + { old_password, new_password }
       → auth_guard 验证
       → 查询当前密码哈希
       → bcrypt 验证旧密码
       → bcrypt 加密新密码
       → 更新数据库
       → 返回成功消息
```

### 4. 上传头像

```
客户端 → POST /api/profile/avatar + multipart/form-data
       → auth_guard 验证
       → 读取文件数据
       → AvatarService 验证（类型+大小）
       → 上传到 MinIO
       → 更新数据库（avatar_url）
       → 返回头像 URL
```

## 🗄️ 数据库字段

在 `users` 表中使用以下字段：

| 字段名 | 类型 | 说明 | 默认值 |
|--------|------|------|--------|
| `user-email` | TEXT | 邮箱地址 | NULL |
| `user-signature` | TEXT | 个性签名 | '' |
| `user-avatar-url` | TEXT | 头像 URL | '' |
| `user-password` | TEXT | 密码哈希 | 必填 |
| `created-at` | TIMESTAMP | 创建时间 | CURRENT_TIMESTAMP |
| `updated-at` | TIMESTAMP | 更新时间 | CURRENT_TIMESTAMP |

**更新触发器**：`updated-at` 字段自动更新（数据库触发器）。

## 🔒 安全特性

### 1. 身份验证
- 所有接口需要有效的 Access Token
- Token 通过 RSA 签名验证
- 用户只能修改自己的信息

### 2. 密码安全
- 密码使用 bcrypt 加密存储
- 修改密码需验证旧密码
- 新密码立即生效

### 3. 文件上传安全
- 文件类型验证（仅允许图片）
- 文件大小限制（5MB）
- 防止路径遍历攻击

### 4. 数据验证
- 使用 `validator` crate 验证数据格式
- 邮箱格式验证
- 字符串长度限制

## 🎯 使用示例

### 在 main.rs 中注册路由

```rust
use huanvae_chat::profile::handlers::routes::profile_routes;
use huanvae_chat::storage::S3Client;

let app = Router::new()
    .merge(profile_routes(db.clone(), s3_client.clone(), auth_state.clone()))
    // ... 其他路由
```

### 在 Handler 中使用

```rust
use crate::auth::middleware::AuthContext;
use crate::profile::services::ProfileService;

pub async fn get_profile(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let user_id = &auth_ctx.user_id;
    
    match ProfileService::get_profile(&state.pool, user_id).await {
        Ok(profile) => (StatusCode::OK, Json(json!({ "data": profile }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
    }
}
```

## 🔧 配置要求

### 环境变量

```env
# 数据库
DATABASE_URL=postgresql://user:pass@localhost:5432/dbname

# MinIO（头像上传）
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin123
MINIO_BUCKET_AVATARS=avatars
MINIO_PUBLIC_URL=http://localhost:9000
```

### 依赖项

```toml
validator = { version = "0.20.0", features = ["derive"] }
bcrypt = "0.17.1"
axum = { version = "0.8.7", features = ["macros", "multipart"] }
```

## 🚀 扩展指南

### 添加新字段

1. **数据库迁移**：
```sql
ALTER TABLE "users" ADD COLUMN "new-field" TEXT DEFAULT '';
```

2. **更新模型**：
```rust
pub struct UpdateProfileRequest {
    pub new_field: Option<String>,  // 新字段
    // ... 其他字段
}
```

3. **更新服务**：
```rust
// 在 profile_service.rs 中更新 SQL
if let Some(new_field) = &request.new_field {
    updates.push(format!(r#""new-field" = ${}"#, args_count));
    args_count += 1;
}
```

### 添加新接口

1. **创建 Handler**：
```rust
// src/profile/handlers/new_feature.rs
pub async fn new_feature(
    State(state): State<ProfileAppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    // 实现逻辑
}
```

2. **注册路由**：
```rust
// src/profile/handlers/routes.rs
Router::new()
    .route("/api/profile/new-feature", post(new_feature))
```

## 🏗️ 设计原则

- **职责分离**：Handlers 处理 HTTP，Services 处理业务逻辑
- **安全优先**：所有接口需要身份验证
- **数据验证**：输入验证在多个层次进行
- **错误处理**：统一使用 `anyhow::Error`
- **复用优先**：复用 `auth` 模块的认证机制
- **RESTful**：遵循 REST API 设计规范

