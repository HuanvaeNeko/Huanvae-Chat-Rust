# Common 模块

公共模块，提供统一的错误类型和 API 响应格式。

## 📂 目录结构

```
common/
├── mod.rs          # 模块入口，导出公共类型
├── errors.rs       # 统一错误类型 AppError
├── response.rs     # 统一响应格式 ApiResponse
└── README.md       # 本文档
```

## 🎯 设计目的

1. **统一错误处理**：消除不同模块间错误类型混用的问题
2. **统一响应格式**：所有 API 返回一致的 JSON 结构
3. **简化 Handler 代码**：直接返回 `Result<ApiResponse<T>, AppError>`

---

## 📦 AppError - 统一错误类型

### 错误分类

| 错误类型 | HTTP 状态码 | 说明 |
|---------|------------|------|
| `Unauthorized` | 401 | 未授权访问 |
| `InvalidToken` | 401 | Token 无效或已过期 |
| `TokenRevoked` | 401 | Token 已被撤销 |
| `Forbidden` | 403 | 权限不足 |
| `NotFound(String)` | 404 | 资源不存在 |
| `BadRequest(String)` | 400 | 请求参数错误 |
| `ValidationError(String)` | 400 | 验证错误 |
| `Conflict(String)` | 409 | 资源冲突 |
| `Internal` | 500 | 内部服务器错误 |
| `Database(String)` | 500 | 数据库错误（详情不暴露给用户）|
| `Storage(String)` | 500 | 存储服务错误（详情不暴露给用户）|

### 自动转换

```rust
// 从 AuthError 转换
impl From<AuthError> for AppError { ... }

// 从 sqlx::Error 转换
impl From<sqlx::Error> for AppError { ... }

// 从 anyhow::Error 转换
impl From<anyhow::Error> for AppError { ... }
```

### 使用示例

```rust
use crate::common::AppError;

// 在 Service 中返回 AppError
pub async fn get_user(&self, id: &str) -> Result<User, AppError> {
    let user = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&self.db)
        .await?  // sqlx::Error 自动转换为 AppError::Database
        .ok_or_else(|| AppError::NotFound("用户".to_string()))?;
    Ok(user)
}

// 在 Handler 中使用
pub async fn get_user_handler(
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<User>>, AppError> {
    let user = service.get_user(&id).await?;
    Ok(Json(ApiResponse::success(user)))
}
```

---

## 📨 ApiResponse - 统一响应格式

### JSON 结构

```json
// 成功响应（带数据）
{
    "success": true,
    "code": 200,
    "data": { ... }
}

// 成功响应（仅消息）
{
    "success": true,
    "code": 200,
    "message": "操作成功"
}

// 成功响应（带数据和消息）
{
    "success": true,
    "code": 200,
    "data": { ... },
    "message": "操作成功"
}

// 错误响应
{
    "success": false,
    "code": 400,
    "error": "参数错误"
}
```

### 构造方法

```rust
use crate::common::ApiResponse;

// 成功响应（带数据）
ApiResponse::success(data)

// 成功响应（带数据和消息）
ApiResponse::success_with_message(data, "操作成功")

// 成功响应（仅消息，无数据）
ApiResponse::ok("操作成功")

// 错误响应
ApiResponse::error(400, "参数错误")

// 从 StatusCode 创建错误响应
ApiResponse::from_error(StatusCode::BAD_REQUEST, "参数错误")
```

### Handler 使用示例

```rust
use crate::common::{ApiResponse, AppError};

// 返回数据
pub async fn get_profile(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ProfileResponse>>, AppError> {
    let profile = state.service.get_profile().await?;
    Ok(Json(ApiResponse::success(profile)))
}

// 返回消息
pub async fn update_profile(
    State(state): State<AppState>,
    Json(req): Json<UpdateRequest>,
) -> Result<ApiResponse<()>, AppError> {
    state.service.update_profile(req).await?;
    Ok(ApiResponse::ok("更新成功"))
}
```

---

## 🔄 迁移指南

### 从 anyhow::Error 迁移

**之前：**
```rust
pub async fn some_service(&self) -> Result<Data, anyhow::Error> {
    // ...
    Err(anyhow::anyhow!("Something went wrong"))
}
```

**之后：**
```rust
use crate::common::AppError;

pub async fn some_service(&self) -> Result<Data, AppError> {
    // ...
    Err(AppError::BadRequest("Something went wrong".to_string()))
}
```

### 从 AuthError 迁移

由于 `AppError` 实现了 `From<AuthError>`，现有代码可以继续使用 `AuthError`，
在需要转换为 `AppError` 的地方会自动转换：

```rust
// 这两种写法都可以
fn handler() -> Result<..., AppError> {
    // AuthError 会自动转换为 AppError
    some_auth_operation()?;
    Ok(...)
}
```

---

## 📋 最佳实践

1. **Service 层使用 `AppError`**：服务层返回 `Result<T, AppError>`
2. **Handler 层返回统一响应**：`Result<Json<ApiResponse<T>>, AppError>`
3. **内部错误不暴露详情**：`Database` 和 `Storage` 错误的详情只记录日志
4. **使用语义化的错误类型**：选择最合适的错误类型，而不是都用 `Internal`

---

## 🧪 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("test data");
        assert!(resp.success);
        assert_eq!(resp.code, 200);
    }

    #[test]
    fn test_app_error_status_code() {
        assert_eq!(
            AppError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
    }
}
```

