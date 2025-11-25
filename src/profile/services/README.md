# Profile Services

业务逻辑层，负责与数据库交互、执行业务规则、处理数据转换。

## 📁 文件列表

- `profile_service.rs` - 个人资料服务实现
- `mod.rs` - 服务模块导出

## 🔧 ProfileService

### 概述

`ProfileService` 提供所有与用户个人资料相关的业务逻辑，包括：
- 查询用户信息
- 更新邮箱和签名
- 修改密码
- 更新头像 URL

**设计特点**：
- 纯静态方法（无状态）
- 异步操作（`async/await`）
- 统一错误处理（`anyhow::Error`）
- 详细日志记录（`tracing`）

---

## 📚 公开方法

### get_profile()

获取用户完整信息（不含密码）。

```rust
pub async fn get_profile(
    pool: &PgPool,
    user_id: &str,
) -> Result<ProfileResponse, anyhow::Error>
```

**参数**：
- `pool`: 数据库连接池引用
- `user_id`: 用户 ID

**返回**：
- `Ok(ProfileResponse)`: 用户信息
- `Err(anyhow::Error)`: 查询失败或用户不存在

**数据库查询**：
```sql
SELECT 
    "user-id",
    "user-nickname",
    "user-email",
    "user-signature",
    "user-avatar-url",
    "admin",
    "created-at",
    "updated-at"
FROM "users"
WHERE "user-id" = $1
```

**使用示例**：
```rust
let profile = ProfileService::get_profile(&pool, "user-123").await?;
println!("User: {}", profile.user_nickname);
```

**注意事项**：
- 时间戳从 `NaiveDateTime` 转换为 `DateTime<Utc>`
- 缺失字段使用默认值
- 不返回密码字段

---

### update_profile()

更新用户的邮箱和/或个性签名。

```rust
pub async fn update_profile(
    pool: &PgPool,
    user_id: &str,
    request: UpdateProfileRequest,
) -> Result<(), anyhow::Error>
```

**参数**：
- `pool`: 数据库连接池引用
- `user_id`: 用户 ID
- `request`: 更新请求（包含 email 和/或 signature）

**返回**：
- `Ok(())`: 更新成功
- `Err(anyhow::Error)`: 更新失败

**特性**：
- **动态 SQL**：仅更新提供的字段
- **至少一个字段**：如果两个字段都为 `None`，返回错误
- **自动时间戳**：数据库触发器自动更新 `updated-at`

**SQL 生成逻辑**：
```rust
// 示例：仅更新 email
UPDATE "users" SET "user-email" = $1 WHERE "user-id" = $2

// 示例：同时更新 email 和 signature
UPDATE "users" SET "user-email" = $1, "user-signature" = $2 WHERE "user-id" = $3
```

**使用示例**：
```rust
let request = UpdateProfileRequest {
    email: Some("new@example.com".to_string()),
    signature: None,
};

ProfileService::update_profile(&pool, "user-123", request).await?;
```

---

### update_password()

修改用户密码。

```rust
pub async fn update_password(
    pool: &PgPool,
    user_id: &str,
    request: UpdatePasswordRequest,
) -> Result<(), anyhow::Error>
```

**参数**：
- `pool`: 数据库连接池引用
- `user_id`: 用户 ID
- `request`: 密码修改请求（包含旧密码和新密码）

**返回**：
- `Ok(())`: 密码修改成功
- `Err("Old password is incorrect")`: 旧密码错误
- `Err(anyhow::Error)`: 其他错误

**处理流程**：
1. 查询当前密码哈希
```sql
SELECT "user-password" FROM "users" WHERE "user-id" = $1
```

2. 使用 bcrypt 验证旧密码
```rust
bcrypt::verify(&request.old_password, &current_hash)?
```

3. 使用 bcrypt 加密新密码
```rust
let new_hash = hash_password(&request.new_password)?;
```

4. 更新数据库
```sql
UPDATE "users" SET "user-password" = $1 WHERE "user-id" = $2
```

**使用示例**：
```rust
let request = UpdatePasswordRequest {
    old_password: "oldpass123".to_string(),
    new_password: "newpass456".to_string(),
};

match ProfileService::update_password(&pool, "user-123", request).await {
    Ok(_) => println!("Password updated"),
    Err(e) if e.to_string().contains("incorrect") => {
        // 旧密码错误，返回 401
    },
    Err(e) => {
        // 其他错误，返回 500
    }
}
```

**安全特性**：
- bcrypt 加密（工作因子默认）
- 旧密码验证防止未授权修改
- 密码哈希不可逆

---

### update_avatar_url()

更新用户头像 URL。

```rust
pub async fn update_avatar_url(
    pool: &PgPool,
    user_id: &str,
    avatar_url: &str,
) -> Result<(), anyhow::Error>
```

**参数**：
- `pool`: 数据库连接池引用
- `user_id`: 用户 ID
- `avatar_url`: 新头像 URL

**返回**：
- `Ok(())`: 更新成功
- `Err(anyhow::Error)`: 更新失败

**数据库操作**：
```sql
UPDATE "users" SET "user-avatar-url" = $1 WHERE "user-id" = $2
```

**使用示例**：
```rust
let url = "http://localhost:9000/avatars/user-123.jpg";
ProfileService::update_avatar_url(&pool, "user-123", url).await?;
```

**调用时机**：
- 在头像上传成功后调用
- 由 `upload_avatar` handler 调用
- URL 应该是完整的公开访问地址

---

## 🔄 典型调用流程

### 获取个人信息

```
Handler (get_profile.rs)
    ↓
ProfileService::get_profile()
    ↓
数据库查询 (SELECT)
    ↓
数据转换 (NaiveDateTime → DateTime<Utc>)
    ↓
返回 ProfileResponse
    ↓
Handler 构建 JSON 响应
```

### 修改密码

```
Handler (update_password.rs)
    ↓
请求验证 (validator)
    ↓
ProfileService::update_password()
    ├─ 查询当前密码哈希
    ├─ bcrypt 验证旧密码
    ├─ bcrypt 加密新密码
    └─ 更新数据库
    ↓
返回成功/失败
    ↓
Handler 构建响应
```

### 上传头像

```
Handler (upload_avatar.rs)
    ↓
读取文件数据 (Multipart)
    ↓
AvatarService::upload_avatar()
    ├─ 验证文件类型和大小
    └─ 上传到 MinIO
    ↓
ProfileService::update_avatar_url()
    └─ 更新数据库
    ↓
返回头像 URL
    ↓
Handler 构建响应
```

---

## 🛡️ 错误处理

### 错误类型

使用 `anyhow::Error` 统一错误类型，便于错误传播。

### 错误日志

```rust
use tracing::{error, info};

// 成功日志
info!("User profile updated: {}", user_id);

// 错误日志
error!("Failed to fetch user profile: {}", e);
```

### Handler 中的错误处理

```rust
match ProfileService::get_profile(&pool, user_id).await {
    Ok(profile) => {
        // 返回 200
        (StatusCode::OK, Json(json!({ "data": profile })))
    },
    Err(e) => {
        // 返回 500
        error!("Failed to get profile: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to fetch profile" }))
        )
    }
}
```

---

## 🗄️ 数据库操作

### 字段映射

数据库使用连字符命名，Rust 代码使用蛇形命名：

| 数据库字段 | Rust 字段 |
|-----------|-----------|
| `user-id` | `user_id` |
| `user-nickname` | `user_nickname` |
| `user-email` | `user_email` |
| `user-signature` | `user_signature` |
| `user-avatar-url` | `user_avatar_url` |
| `user-password` | - (不返回) |
| `created-at` | `created_at` |
| `updated-at` | `updated_at` |

### 时间戳处理

```rust
// 数据库返回 NaiveDateTime (无时区)
let (created_at, updated_at): (Option<NaiveDateTime>, Option<NaiveDateTime>) = ...;

// 转换为 DateTime<Utc>
created_at.map(|dt| dt.and_utc()).unwrap_or_default()
```

### 动态 SQL 构建

```rust
let mut updates = Vec::new();
let mut args_count = 1;

if request.email.is_some() {
    updates.push(format!(r#""user-email" = ${}"#, args_count));
    args_count += 1;
}

if request.signature.is_some() {
    updates.push(format!(r#""user-signature" = ${}"#, args_count));
    args_count += 1;
}

let update_clause = updates.join(", ");
let query_str = format!(
    r#"UPDATE "users" SET {} WHERE "user-id" = ${}"#,
    update_clause, args_count
);
```

---

## 🚀 扩展指南

### 添加新的服务方法

```rust
impl ProfileService {
    /// 更新用户昵称
    pub async fn update_nickname(
        pool: &PgPool,
        user_id: &str,
        nickname: &str,
    ) -> Result<(), anyhow::Error> {
        sqlx::query(
            r#"UPDATE "users" SET "user-nickname" = $1 WHERE "user-id" = $2"#
        )
        .bind(nickname)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to update nickname: {}", e);
            anyhow::anyhow!("Failed to update nickname")
        })?;

        info!("Nickname updated for user: {}", user_id);
        Ok(())
    }
}
```

### 使用事务

```rust
pub async fn complex_update(
    pool: &PgPool,
    user_id: &str,
    // ... 参数
) -> Result<(), anyhow::Error> {
    let mut tx = pool.begin().await?;

    // 操作 1
    sqlx::query("...").execute(&mut *tx).await?;
    
    // 操作 2
    sqlx::query("...").execute(&mut *tx).await?;
    
    tx.commit().await?;
    Ok(())
}
```

---

## 🎯 设计原则

- **单一职责**：每个方法只做一件事
- **无状态**：使用静态方法，依赖注入
- **错误传播**：使用 `?` 操作符简化错误处理
- **日志完整**：记录关键操作和错误
- **类型安全**：充分利用 Rust 类型系统
- **异步优先**：所有 IO 操作使用 async

