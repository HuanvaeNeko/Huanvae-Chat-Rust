use crate::auth::errors::AuthError;
use crate::friends_messages::models::{Message, MessageResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// 消息服务
pub struct MessageService {
    db: PgPool,
}

impl MessageService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 生成会话UUID（双方用户ID排序后组合）
    pub fn generate_conversation_uuid(user_id_1: &str, user_id_2: &str) -> String {
        let mut ids = vec![user_id_1, user_id_2];
        ids.sort();
        format!("conv-{}-{}", ids[0], ids[1])
    }

    /// 验证双方是否为好友关系
    pub async fn verify_friendship(&self, user_id: &str, friend_id: &str) -> Result<bool, AuthError> {
        let user_friends: String = sqlx::query_scalar(
            r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

        // 简单解析TEXT字段检查好友关系
        Ok(user_friends.contains(&format!("friend-id:{}", friend_id))
            && user_friends.contains("status:active"))
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        sender_id: &str,
        receiver_id: &str,
        message_content: &str,
        message_type: &str,
        file_url: Option<String>,
        file_size: Option<i64>,
    ) -> Result<(String, String), AuthError> {
        // 1. 验证好友关系
        if !self.verify_friendship(sender_id, receiver_id).await? {
            return Err(AuthError::BadRequest("不是好友关系，无法发送消息".to_string()));
        }

        // 2. 生成消息UUID和会话UUID
        let message_uuid = Uuid::new_v4().to_string();
        let conversation_uuid = Self::generate_conversation_uuid(sender_id, receiver_id);
        let send_time = Utc::now();

        // 3. 插入消息到数据库
        sqlx::query(
            r#"
            INSERT INTO "friend-messages" (
                "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                "message-content", "message-type", "file-url", "file-size", "send-time"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&message_uuid)
        .bind(&conversation_uuid)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(message_content)
        .bind(message_type)
        .bind(&file_url)
        .bind(file_size)
        .bind(send_time)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("插入消息失败: {}", e);
            AuthError::InternalServerError
        })?;

        Ok((message_uuid, send_time.to_rfc3339()))
    }

    /// 获取消息列表
    pub async fn get_messages(
        &self,
        user_id: &str,
        friend_id: &str,
        before_uuid: Option<String>,
        limit: i32,
    ) -> Result<(Vec<MessageResponse>, bool), AuthError> {
        // 1. 验证好友关系
        if !self.verify_friendship(user_id, friend_id).await? {
            return Err(AuthError::BadRequest("不是好友关系".to_string()));
        }

        // 2. 生成会话UUID
        let conversation_uuid = Self::generate_conversation_uuid(user_id, friend_id);

        // 3. 查询消息
        let messages: Vec<Message> = if let Some(before_uuid) = before_uuid {
            // 分页查询：从指定消息之前查询
            sqlx::query_as(
                r#"
                SELECT "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                       "message-content", "message-type", "file-url", "file-size", "send-time",
                       "is-deleted-by-sender", "is-deleted-by-receiver"
                FROM "friend-messages"
                WHERE "conversation-uuid" = $1
                  AND "send-time" < (
                      SELECT "send-time" FROM "friend-messages" WHERE "message-uuid" = $2
                  )
                  AND (
                      ("sender-id" = $3 AND "is-deleted-by-sender" = false) OR
                      ("receiver-id" = $3 AND "is-deleted-by-receiver" = false)
                  )
                ORDER BY "send-time" DESC
                LIMIT $4
                "#,
            )
            .bind(&conversation_uuid)
            .bind(&before_uuid)
            .bind(user_id)
            .bind(limit + 1)  // 多查一条用于判断是否有更多
            .fetch_all(&self.db)
            .await
        } else {
            // 查询最新消息
            sqlx::query_as(
                r#"
                SELECT "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                       "message-content", "message-type", "file-url", "file-size", "send-time",
                       "is-deleted-by-sender", "is-deleted-by-receiver"
                FROM "friend-messages"
                WHERE "conversation-uuid" = $1
                  AND (
                      ("sender-id" = $2 AND "is-deleted-by-sender" = false) OR
                      ("receiver-id" = $2 AND "is-deleted-by-receiver" = false)
                  )
                ORDER BY "send-time" DESC
                LIMIT $3
                "#,
            )
            .bind(&conversation_uuid)
            .bind(user_id)
            .bind(limit + 1)
            .fetch_all(&self.db)
            .await
        }
        .map_err(|e| {
            tracing::error!("查询消息失败: {}", e);
            AuthError::InternalServerError
        })?;

        // 4. 判断是否还有更多消息
        let has_more = messages.len() > limit as usize;
        let messages: Vec<MessageResponse> = messages
            .into_iter()
            .take(limit as usize)
            .map(MessageResponse::from)
            .collect();

        Ok((messages, has_more))
    }

    /// 删除消息（软删除）
    pub async fn delete_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AuthError> {
        // 1. 查询消息
        let message: Option<(String, String)> = sqlx::query_as(
            r#"SELECT "sender-id", "receiver-id" FROM "friend-messages" WHERE "message-uuid" = $1"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

        let (sender_id, receiver_id) = message.ok_or(AuthError::BadRequest("消息不存在".to_string()))?;

        // 2. 根据用户身份标记删除
        if user_id == sender_id {
            // 发送者删除
            sqlx::query(
                r#"UPDATE "friend-messages" SET "is-deleted-by-sender" = true WHERE "message-uuid" = $1"#,
            )
            .bind(message_uuid)
            .execute(&self.db)
            .await
        } else if user_id == receiver_id {
            // 接收者删除
            sqlx::query(
                r#"UPDATE "friend-messages" SET "is-deleted-by-receiver" = true WHERE "message-uuid" = $1"#,
            )
            .bind(message_uuid)
            .execute(&self.db)
            .await
        } else {
            return Err(AuthError::Forbidden);
        }
        .map_err(|_| AuthError::InternalServerError)?;

        Ok(())
    }

    /// 撤回消息（双方都标记为已删除）
    pub async fn recall_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AuthError> {
        // 1. 查询消息
        let message: Option<(String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT "sender-id", "send-time" FROM "friend-messages" WHERE "message-uuid" = $1"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

        let (sender_id, send_time) = message.ok_or(AuthError::BadRequest("消息不存在".to_string()))?;

        // 2. 只有发送者可以撤回
        if user_id != sender_id {
            return Err(AuthError::Forbidden);
        }

        // 3. 检查是否超过2分钟
        let now = Utc::now();
        let duration = now.signed_duration_since(send_time);
        if duration.num_minutes() > 2 {
            return Err(AuthError::BadRequest("消息发送超过2分钟，无法撤回".to_string()));
        }

        // 4. 标记双方都已删除（撤回）
        sqlx::query(
            r#"
            UPDATE "friend-messages" 
            SET "is-deleted-by-sender" = true, "is-deleted-by-receiver" = true
            WHERE "message-uuid" = $1
            "#,
        )
        .bind(message_uuid)
        .execute(&self.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

        Ok(())
    }
}

