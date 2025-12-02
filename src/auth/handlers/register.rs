use crate::auth::{
    models::{RegisterRequest, UserResponse},
    services::TokenService,
    utils::{hash_password, validate_email, validate_nickname, validate_password_strength},
};
use crate::common::AppError;
use axum::{extract::State, Json};
use sqlx::PgPool;
use std::sync::Arc;

/// 注册处理器状态
#[derive(Clone)]
pub struct RegisterState {
    pub db: PgPool,
    pub token_service: Arc<TokenService>,
}

/// 用户注册
pub async fn register_handler(
    State(state): State<RegisterState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // 1. 验证输入
    if let Some(email) = &req.email {
        validate_email(email)?;
    }
    validate_nickname(&req.nickname)?;
    validate_password_strength(&req.password)?;

    // 2. 检查用户ID是否已存在
    let existing_id: Option<(String,)> = sqlx::query_as(
        r#"SELECT "user-id" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&req.user_id)
    .fetch_optional(&state.db)
    .await?;

    if existing_id.is_some() {
        return Err(AppError::BadRequest("用户ID已存在".to_string()));
    }

    // 3. 检查邮箱是否已存在（如果提供）
    if let Some(email) = &req.email {
        let existing_user: Option<(String,)> = sqlx::query_as(
            r#"SELECT "user-id" FROM "users" WHERE "user-email" = $1"#,
        )
        .bind(email)
        .fetch_optional(&state.db)
        .await?;

        if existing_user.is_some() {
            return Err(AppError::Conflict("用户已存在".to_string()));
        }
    }

    // 4. 哈希密码
    let password_hash = hash_password(&req.password)?;

    // 5. 使用用户提供的ID插入数据库
    sqlx::query(
        r#"
        INSERT INTO "users" (
            "user-id", "user-nickname", "user-password", "user-email", "admin"
        )
        VALUES ($1, $2, $3, $4, 'false')
        "#,
    )
    .bind(&req.user_id)
    .bind(&req.nickname)
    .bind(&password_hash)
    .bind(&req.email.clone().unwrap_or("未填写邮箱".to_string()))
    .execute(&state.db)
    .await?;

    // 6. 查询新创建的用户
    let user: crate::auth::models::User = sqlx::query_as(
        r#"SELECT * FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&req.user_id)
    .fetch_one(&state.db)
    .await?;

    tracing::info!("✅ 用户注册成功: {} ({})", req.nickname, req.email.clone().unwrap_or_default());

    Ok(Json(UserResponse::from(user)))
}

