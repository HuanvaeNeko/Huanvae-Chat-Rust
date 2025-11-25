# Profile Models

数据模型定义，包括请求体（Request）和响应体（Response）的结构定义。

## 📁 文件列表

- `request.rs` - 请求体模型
- `response.rs` - 响应体模型
- `mod.rs` - 模型导出

## 📥 请求模型 (`request.rs`)

### UpdateProfileRequest

更新个人信息的请求体。

```rust
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    
    #[validate(length(max = 200, message = "Signature too long (max 200 characters)"))]
    pub signature: Option<String>,
}
```

**字段说明**：

| 字段 | 类型 | 必填 | 验证规则 | 说明 |
|------|------|------|----------|------|
| `email` | `Option<String>` | ❌ | 邮箱格式 | 新邮箱地址 |
| `signature` | `Option<String>` | ❌ | 最长 200 字符 | 个性签名 |

**验证器**：使用 `validator` crate 的 `Validate` trait

**使用示例**：
```rust
let request = UpdateProfileRequest {
    email: Some("new@example.com".to_string()),
    signature: Some("Hello, world!".to_string()),
};

// 验证
request.validate()?;
```

**JSON 示例**：
```json
{
  "email": "new@example.com",
  "signature": "Hello, world!"
}
```

**验证错误示例**：
```json
{
  "error": "Validation error: email: Invalid email format"
}
```

---

### UpdatePasswordRequest

修改密码的请求体。

```rust
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdatePasswordRequest {
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub old_password: String,
    
    #[validate(length(min = 6, max = 100, message = "Password must be 6-100 characters"))]
    pub new_password: String,
}
```

**字段说明**：

| 字段 | 类型 | 必填 | 验证规则 | 说明 |
|------|------|------|----------|------|
| `old_password` | `String` | ✅ | 至少 6 字符 | 当前密码 |
| `new_password` | `String` | ✅ | 6-100 字符 | 新密码 |

**使用示例**：
```rust
let request = UpdatePasswordRequest {
    old_password: "oldpass123".to_string(),
    new_password: "newpass456".to_string(),
};

request.validate()?;
```

**JSON 示例**：
```json
{
  "old_password": "oldpass123",
  "new_password": "newpass456"
}
```

**安全考虑**：
- 密码在传输时应使用 HTTPS
- 旧密码需在服务端验证
- 新密码使用 bcrypt 加密存储

---

## 📤 响应模型 (`response.rs`)

### ProfileResponse

用户完整信息响应体（不含密码）。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileResponse {
    #[serde(rename = "user_id")]
    pub user_id: String,
    
    #[serde(rename = "user_nickname")]
    pub user_nickname: String,
    
    #[serde(rename = "user_email")]
    pub user_email: Option<String>,
    
    #[serde(rename = "user_signature")]
    pub user_signature: Option<String>,
    
    #[serde(rename = "user_avatar_url")]
    pub user_avatar_url: Option<String>,
    
    #[serde(rename = "admin")]
    pub admin: String,
    
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `user_id` | `String` | 用户唯一标识 |
| `user_nickname` | `String` | 用户昵称 |
| `user_email` | `Option<String>` | 邮箱地址 |
| `user_signature` | `Option<String>` | 个性签名 |
| `user_avatar_url` | `Option<String>` | 头像 URL |
| `admin` | `String` | 是否管理员 ("true"/"false") |
| `created_at` | `DateTime<Utc>` | 账号创建时间（UTC） |
| `updated_at` | `DateTime<Utc>` | 最后更新时间（UTC） |

**JSON 示例**：
```json
{
  "user_id": "testuser001",
  "user_nickname": "测试用户",
  "user_email": "test@example.com",
  "user_signature": "Hello, world!",
  "user_avatar_url": "http://localhost:9000/avatars/testuser001.jpg",
  "admin": "false",
  "created_at": "2025-11-25T11:00:02.221791Z",
  "updated_at": "2025-11-25T11:03:40.730806Z"
}
```

**特性**：
- 使用 `serde(rename)` 映射到蛇形命名
- 时间字段使用 ISO 8601 格式
- **不包含密码字段**（安全考虑）

---

### AvatarUploadResponse

头像上传成功响应体。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
    pub message: String,
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `avatar_url` | `String` | 头像公开访问 URL |
| `message` | `String` | 成功消息 |

**JSON 示例**：
```json
{
  "avatar_url": "http://localhost:9000/avatars/testuser001.jpg",
  "message": "Avatar uploaded successfully"
}
```

**URL 格式**：
```
{MINIO_PUBLIC_URL}/avatars/{user_id}.{extension}
```

**特性**：
- URL 可直接访问（公开读取）
- 自动覆盖旧头像（同名文件）
- 支持的扩展名：jpg, jpeg, png, gif, webp

---

## 🎯 使用指南

### 在 Handler 中使用

```rust
use crate::profile::models::{UpdateProfileRequest, ProfileResponse};
use axum::Json;

pub async fn update_profile(
    Json(request): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    // 验证
    request.validate()?;
    
    // 处理业务逻辑
    // ...
    
    // 返回响应
    (StatusCode::OK, Json(json!({ "message": "Success" })))
}
```

### 验证数据

```rust
use validator::Validate;

match request.validate() {
    Ok(_) => {
        // 验证通过
    },
    Err(e) => {
        // 返回 400 错误
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Validation error: {}", e) }))
        );
    }
}
```

### 构建响应

```rust
use chrono::Utc;

let response = ProfileResponse {
    user_id: "user-123".to_string(),
    user_nickname: "John Doe".to_string(),
    user_email: Some("john@example.com".to_string()),
    user_signature: Some("Hello!".to_string()),
    user_avatar_url: Some("http://example.com/avatar.jpg".to_string()),
    admin: "false".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

(StatusCode::OK, Json(json!({ "data": response })))
```

## 🔧 扩展模型

### 添加新字段

1. **更新请求模型**：
```rust
pub struct UpdateProfileRequest {
    pub email: Option<String>,
    pub signature: Option<String>,
    pub phone: Option<String>,  // 新字段
}
```

2. **添加验证**：
```rust
#[validate(length(min = 10, max = 15, message = "Invalid phone number"))]
pub phone: Option<String>,
```

3. **更新响应模型**：
```rust
pub struct ProfileResponse {
    // ... 其他字段
    pub phone: Option<String>,  // 新字段
}
```

### 创建新请求模型

```rust
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct NewFeatureRequest {
    #[validate(length(min = 1, max = 100))]
    pub field1: String,
    
    pub field2: Option<i32>,
}
```

## 📋 验证规则参考

常用验证器（来自 `validator` crate）：

| 验证器 | 说明 | 示例 |
|--------|------|------|
| `email` | 邮箱格式 | `#[validate(email)]` |
| `length` | 字符串长度 | `#[validate(length(min = 6, max = 100))]` |
| `range` | 数值范围 | `#[validate(range(min = 0, max = 150))]` |
| `url` | URL 格式 | `#[validate(url)]` |
| `custom` | 自定义验证 | `#[validate(custom = "my_validator")]` |

## 🎨 设计原则

- **类型安全**：充分利用 Rust 类型系统
- **验证完整**：在数据模型层就进行验证
- **清晰命名**：字段名清晰表达含义
- **可序列化**：所有模型支持 JSON 序列化
- **安全第一**：响应中不包含敏感信息

