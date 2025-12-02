use crate::auth::errors::AuthError;
use crate::auth::middleware::AuthContext;
use crate::friends::models::{
    ApproveFriendRequest, RejectFriendRequest, RemoveFriendRequest, SubmitFriendRequest,
    SubmitFriendResponse,
};
use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// 好友服务
#[derive(Clone)]
pub struct FriendsState {
    pub db: PgPool,
}

impl FriendsState {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

fn ensure_user_id_matches_token(req_user_id: &str, auth: &AuthContext) -> Result<(), AuthError> {
    if req_user_id != auth.user_id {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}

/// 提交好友请求
pub async fn submit_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: SubmitFriendRequest,
) -> Result<SubmitFriendResponse, AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 检查目标用户是否存在
    let target_exists: Option<(String,)> = sqlx::query_as(
        r#"SELECT "user-id" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询目标用户失败: {}", e);
        AuthError::InternalServerError
    })?;

    if target_exists.is_none() {
        return Err(AuthError::BadRequest("目标用户不存在".to_string()));
    }

    // 检查是否已经是好友
    let already_friends: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM friendships 
           WHERE user_id = $1 AND friend_id = $2 AND status = 'active'"#,
    )
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询好友关系失败: {}", e);
        AuthError::InternalServerError
    })?;

    if already_friends.is_some() {
        return Err(AuthError::BadRequest("已经是好友关系".to_string()));
    }

    // 检查是否有对方发来的待处理请求（自动互通过）
    let reverse_request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM friend_requests 
           WHERE from_user_id = $1 AND to_user_id = $2 AND status = 'pending'"#,
    )
    .bind(&body.target_user_id)
    .bind(&body.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询反向好友请求失败: {}", e);
        AuthError::InternalServerError
    })?;

    if let Some((reverse_id,)) = reverse_request {
        // 🔒 使用事务处理自动互通过
        let mut tx = state.db.begin().await
            .map_err(|e| {
                tracing::error!("开始事务失败 [自动互通过]: {}", e);
                AuthError::InternalServerError
            })?;

        // 更新对方请求为已同意
        sqlx::query(
            r#"UPDATE friend_requests SET status = 'approved', "updated-at" = $1 WHERE id = $2"#,
        )
        .bind(Utc::now())
        .bind(reverse_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("更新好友请求状态失败 [自动互通过]: {}", e);
            AuthError::InternalServerError
        })?;

        // 建立双向好友关系
        create_friendship_tx(&mut tx, &body.target_user_id, &body.user_id).await?;
        create_friendship_tx(&mut tx, &body.user_id, &body.target_user_id).await?;

        // 提交事务
        tx.commit().await
            .map_err(|e| {
                tracing::error!("提交事务失败 [自动互通过]: {}", e);
                AuthError::InternalServerError
            })?;

        return Ok(SubmitFriendResponse {
            request_id: reverse_id.to_string(),
        });
    }

    // 检查是否已有待处理的请求
    let existing_request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM friend_requests 
           WHERE from_user_id = $1 AND to_user_id = $2 AND status = 'pending'"#,
    )
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询已有好友请求失败: {}", e);
        AuthError::InternalServerError
    })?;

    if existing_request.is_some() {
        return Err(AuthError::BadRequest("已有待处理的好友请求".to_string()));
    }

    // 创建新的好友请求
    let request_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO friend_requests (id, from_user_id, to_user_id, message, status, "created-at", "updated-at")
           VALUES ($1, $2, $3, $4, 'pending', $5, $5)"#,
    )
    .bind(request_id)
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .bind(&body.reason.unwrap_or_default())
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("创建好友请求失败: {}", e);
        AuthError::InternalServerError
    })?;

    Ok(SubmitFriendResponse {
        request_id: request_id.to_string(),
    })
}

/// 同意好友请求
pub async fn approve_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: ApproveFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 查找待处理的请求
    let request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM friend_requests 
           WHERE from_user_id = $1 AND to_user_id = $2 AND status = 'pending'"#,
    )
    .bind(&body.applicant_user_id)
    .bind(&body.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("查询待处理好友请求失败: {}", e);
        AuthError::InternalServerError
    })?;

    let (request_id,) = request.ok_or(AuthError::BadRequest("好友请求不存在".to_string()))?;

    // 🔒 使用事务处理同意请求
    let mut tx = state.db.begin().await
        .map_err(|e| {
            tracing::error!("开始事务失败 [同意好友请求]: {}", e);
            AuthError::InternalServerError
        })?;

    // 更新请求状态
    sqlx::query(
        r#"UPDATE friend_requests SET status = 'approved', "updated-at" = $1 WHERE id = $2"#,
    )
    .bind(Utc::now())
    .bind(request_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("更新好友请求状态失败 [同意]: {}", e);
        AuthError::InternalServerError
    })?;

    // 建立双向好友关系
    create_friendship_tx(&mut tx, &body.applicant_user_id, &body.user_id).await?;
    create_friendship_tx(&mut tx, &body.user_id, &body.applicant_user_id).await?;

    // 提交事务
    tx.commit().await
        .map_err(|e| {
            tracing::error!("提交事务失败 [同意好友请求]: {}", e);
            AuthError::InternalServerError
        })?;

    Ok(())
}

/// 拒绝好友请求
pub async fn reject_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: RejectFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 更新请求状态为拒绝（单个操作，不需要事务）
    let result = sqlx::query(
        r#"UPDATE friend_requests 
           SET status = 'rejected', reject_reason = $1, "updated-at" = $2 
           WHERE from_user_id = $3 AND to_user_id = $4 AND status = 'pending'"#,
    )
    .bind(&body.reject_reason)
    .bind(Utc::now())
    .bind(&body.applicant_user_id)
    .bind(&body.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("更新好友请求状态失败 [拒绝]: {}", e);
        AuthError::InternalServerError
    })?;

    if result.rows_affected() == 0 {
        return Err(AuthError::BadRequest("好友请求不存在".to_string()));
    }

    Ok(())
}

/// 删除好友
pub async fn remove_friend(
    state: &FriendsState,
    auth: &AuthContext,
    body: RemoveFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    let now = Utc::now();

    // 🔒 使用事务处理删除好友（保证双向一致性）
    let mut tx = state.db.begin().await
        .map_err(|e| {
            tracing::error!("开始事务失败 [删除好友]: {}", e);
            AuthError::InternalServerError
        })?;

    // 更新用户方的好友关系状态为 ended
    sqlx::query(
        r#"UPDATE friendships 
           SET status = 'ended', end_time = $1, end_reason = $2 
           WHERE user_id = $3 AND friend_id = $4 AND status = 'active'"#,
    )
    .bind(now)
    .bind(&body.remove_reason)
    .bind(&body.user_id)
    .bind(&body.friend_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("更新好友关系状态失败 [用户方]: {}", e);
        AuthError::InternalServerError
    })?;

    // 更新好友方的好友关系状态为 ended
    sqlx::query(
        r#"UPDATE friendships 
           SET status = 'ended', end_time = $1, end_reason = $2 
           WHERE user_id = $3 AND friend_id = $4 AND status = 'active'"#,
    )
    .bind(now)
    .bind(&body.remove_reason)
    .bind(&body.friend_user_id)
    .bind(&body.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("更新好友关系状态失败 [好友方]: {}", e);
        AuthError::InternalServerError
    })?;

    // 提交事务
    tx.commit().await
        .map_err(|e| {
            tracing::error!("提交事务失败 [删除好友]: {}", e);
            AuthError::InternalServerError
        })?;

    Ok(())
}

/// 创建好友关系记录（事务版本）
async fn create_friendship_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: &str,
    friend_id: &str,
) -> Result<(), AuthError> {
    sqlx::query(
        r#"INSERT INTO friendships (user_id, friend_id, status, add_time)
           VALUES ($1, $2, 'active', $3)
           ON CONFLICT (user_id, friend_id) DO UPDATE SET status = 'active', add_time = $3, end_time = NULL, end_reason = NULL"#,
    )
    .bind(user_id)
    .bind(friend_id)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!("创建好友关系失败 [user_id={}, friend_id={}]: {}", user_id, friend_id, e);
        AuthError::InternalServerError
    })?;

    Ok(())
}

/// 验证好友关系（供其他模块调用）
pub async fn verify_friendship(db: &PgPool, user_id: &str, friend_id: &str) -> Result<bool, AuthError> {
    let result: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM friendships 
           WHERE user_id = $1 AND friend_id = $2 AND status = 'active'"#,
    )
    .bind(user_id)
    .bind(friend_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!("验证好友关系失败 [user_id={}, friend_id={}]: {}", user_id, friend_id, e);
        AuthError::InternalServerError
    })?;

    Ok(result.is_some())
}
