use crate::common::AppError;
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
pub struct FriendsService {
    pub db: PgPool,
}

impl FriendsService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

fn ensure_user_id_matches_token(req_user_id: &str, auth: &AuthContext) -> Result<(), AppError> {
    if req_user_id != auth.user_id {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// 提交好友请求
pub async fn submit_request(
    state: &FriendsService,
    auth: &AuthContext,
    body: SubmitFriendRequest,
) -> Result<SubmitFriendResponse, AppError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 检查目标用户是否存在
    let target_exists: Option<(String,)> = sqlx::query_as(
        r#"SELECT "user-id" FROM "users" WHERE "user-id" = $1"#,
    )
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询目标用户失败: {}", e)))?;

    if target_exists.is_none() {
        return Err(AppError::BadRequest("目标用户不存在".to_string()));
    }

    // 检查是否已经是好友
    let already_friends: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "friendships" 
           WHERE "user-id" = $1 AND "friend-id" = $2 AND "status" = 'active'"#,
    )
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询好友关系失败: {}", e)))?;

    if already_friends.is_some() {
        return Err(AppError::BadRequest("已经是好友关系".to_string()));
    }

    // 检查是否有对方发来的待处理请求（自动互通过）
    let reverse_request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "friend-requests" 
           WHERE "from-user-id" = $1 AND "to-user-id" = $2 AND "status" = 'pending'"#,
    )
    .bind(&body.target_user_id)
    .bind(&body.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询反向好友请求失败: {}", e)))?;

    if let Some((reverse_id,)) = reverse_request {
        // 🔒 使用事务处理自动互通过
        let mut tx = state.db.begin().await
            .map_err(|e| AppError::Database(format!("开始事务失败 [自动互通过]: {}", e)))?;

        // 更新对方请求为已同意
        sqlx::query(
            r#"UPDATE "friend-requests" SET "status" = 'approved', "updated-at" = $1 WHERE "id" = $2"#,
        )
        .bind(Utc::now())
        .bind(reverse_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(format!("更新好友请求状态失败 [自动互通过]: {}", e)))?;

        // 建立双向好友关系
        create_friendship_tx(&mut tx, &body.target_user_id, &body.user_id).await?;
        create_friendship_tx(&mut tx, &body.user_id, &body.target_user_id).await?;

        // 提交事务
        tx.commit().await
            .map_err(|e| AppError::Database(format!("提交事务失败 [自动互通过]: {}", e)))?;

        return Ok(SubmitFriendResponse {
            request_id: reverse_id.to_string(),
        });
    }

    // 检查是否已有待处理的请求
    let existing_request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "friend-requests" 
           WHERE "from-user-id" = $1 AND "to-user-id" = $2 AND "status" = 'pending'"#,
    )
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询已有好友请求失败: {}", e)))?;

    if existing_request.is_some() {
        return Err(AppError::BadRequest("已有待处理的好友请求".to_string()));
    }

    // 创建新的好友请求
    let request_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO "friend-requests" ("id", "from-user-id", "to-user-id", "message", "status", "created-at", "updated-at")
           VALUES ($1, $2, $3, $4, 'pending', $5, $5)"#,
    )
    .bind(request_id)
    .bind(&body.user_id)
    .bind(&body.target_user_id)
    .bind(&body.reason.unwrap_or_default())
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("创建好友请求失败: {}", e)))?;

    Ok(SubmitFriendResponse {
        request_id: request_id.to_string(),
    })
}

/// 同意好友请求
pub async fn approve_request(
    state: &FriendsService,
    auth: &AuthContext,
    body: ApproveFriendRequest,
) -> Result<(), AppError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 查找待处理的请求
    let request: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "friend-requests" 
           WHERE "from-user-id" = $1 AND "to-user-id" = $2 AND "status" = 'pending'"#,
    )
    .bind(&body.applicant_user_id)
    .bind(&body.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("查询待处理好友请求失败: {}", e)))?;

    let (request_id,) = request.ok_or(AppError::BadRequest("好友请求不存在".to_string()))?;

    // 🔒 使用事务处理同意请求
    let mut tx = state.db.begin().await
        .map_err(|e| AppError::Database(format!("开始事务失败 [同意好友请求]: {}", e)))?;

    // 更新请求状态
    sqlx::query(
        r#"UPDATE "friend-requests" SET "status" = 'approved', "updated-at" = $1 WHERE "id" = $2"#,
    )
    .bind(Utc::now())
    .bind(request_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(format!("更新好友请求状态失败 [同意]: {}", e)))?;

    // 建立双向好友关系
    create_friendship_tx(&mut tx, &body.applicant_user_id, &body.user_id).await?;
    create_friendship_tx(&mut tx, &body.user_id, &body.applicant_user_id).await?;

    // 提交事务
    tx.commit().await
        .map_err(|e| AppError::Database(format!("提交事务失败 [同意好友请求]: {}", e)))?;

    Ok(())
}

/// 拒绝好友请求
pub async fn reject_request(
    state: &FriendsService,
    auth: &AuthContext,
    body: RejectFriendRequest,
) -> Result<(), AppError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    // 更新请求状态为拒绝（单个操作，不需要事务）
    let result = sqlx::query(
        r#"UPDATE "friend-requests" 
           SET "status" = 'rejected', "reject-reason" = $1, "updated-at" = $2 
           WHERE "from-user-id" = $3 AND "to-user-id" = $4 AND "status" = 'pending'"#,
    )
    .bind(&body.reject_reason)
    .bind(Utc::now())
    .bind(&body.applicant_user_id)
    .bind(&body.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Database(format!("更新好友请求状态失败 [拒绝]: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("好友请求不存在".to_string()));
    }

    Ok(())
}

/// 删除好友
pub async fn remove_friend(
    state: &FriendsService,
    auth: &AuthContext,
    body: RemoveFriendRequest,
) -> Result<(), AppError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    let now = Utc::now();

    // 🔒 使用事务处理删除好友（保证双向一致性）
    let mut tx = state.db.begin().await
        .map_err(|e| AppError::Database(format!("开始事务失败 [删除好友]: {}", e)))?;

    // 更新用户方的好友关系状态为 ended
    sqlx::query(
        r#"UPDATE "friendships" 
           SET "status" = 'ended', "end-time" = $1, "end-reason" = $2 
           WHERE "user-id" = $3 AND "friend-id" = $4 AND "status" = 'active'"#,
    )
    .bind(now)
    .bind(&body.remove_reason)
    .bind(&body.user_id)
    .bind(&body.friend_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(format!("更新好友关系状态失败 [用户方]: {}", e)))?;

    // 更新好友方的好友关系状态为 ended
    sqlx::query(
        r#"UPDATE "friendships" 
           SET "status" = 'ended', "end-time" = $1, "end-reason" = $2 
           WHERE "user-id" = $3 AND "friend-id" = $4 AND "status" = 'active'"#,
    )
    .bind(now)
    .bind(&body.remove_reason)
    .bind(&body.friend_user_id)
    .bind(&body.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(format!("更新好友关系状态失败 [好友方]: {}", e)))?;

    // 提交事务
    tx.commit().await
        .map_err(|e| AppError::Database(format!("提交事务失败 [删除好友]: {}", e)))?;

    Ok(())
}

/// 创建好友关系记录（事务版本）
async fn create_friendship_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: &str,
    friend_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO "friendships" ("user-id", "friend-id", "status", "add-time")
           VALUES ($1, $2, 'active', $3)
           ON CONFLICT ("user-id", "friend-id") DO UPDATE SET "status" = 'active', "add-time" = $3, "end-time" = NULL, "end-reason" = NULL"#,
    )
    .bind(user_id)
    .bind(friend_id)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(format!("创建好友关系失败 [user_id={}, friend_id={}]: {}", user_id, friend_id, e)))?;

    Ok(())
}

/// 验证好友关系（供其他模块调用）
pub async fn verify_friendship(db: &PgPool, user_id: &str, friend_id: &str) -> Result<bool, AppError> {
    let result: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT "id" FROM "friendships" 
           WHERE "user-id" = $1 AND "friend-id" = $2 AND "status" = 'active'"#,
    )
    .bind(user_id)
    .bind(friend_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::Database(format!("验证好友关系失败 [user_id={}, friend_id={}]: {}", user_id, friend_id, e)))?;

    Ok(result.is_some())
}
