use crate::common::{generate_conversation_uuid, AppError};
use crate::config::message_config;
use crate::friends::services::verify_friendship;
use crate::friends_messages::models::{Message, MessageResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// 消息服务
#[derive(Clone)]
pub struct MessageService {
    db: PgPool,
}

impl MessageService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        sender_id: &str,
        receiver_id: &str,
        message_content: &str,
        message_type: &str,
        file_uuid: Option<String>,
        file_url: Option<String>,
        file_size: Option<i64>,
    ) -> Result<(String, String), AppError> {
        // 1. 验证好友关系
        if !verify_friendship(&self.db, sender_id, receiver_id).await? {
            return Err(AppError::BadRequest("不是好友关系，无法发送消息".to_string()));
        }

        // 2. 生成消息UUID和会话UUID
        let message_uuid = Uuid::new_v4().to_string();
        let conversation_uuid = generate_conversation_uuid(sender_id, receiver_id);
        let send_time = Utc::now();

        // 3. 插入消息到数据库（使用 ON CONFLICT 处理UUID冲突）
        sqlx::query(
            r#"
            INSERT INTO "friend-messages" (
                "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                "message-content", "message-type", "file-uuid", "file-url", "file-size", "send-time"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT ("message-uuid") DO NOTHING
            "#,
        )
        .bind(&message_uuid)
        .bind(&conversation_uuid)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(message_content)
        .bind(message_type)
        .bind(&file_uuid)
        .bind(&file_url)
        .bind(file_size)
        .bind(send_time)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("插入消息失败: {}", e)))?;

        Ok((message_uuid, send_time.to_rfc3339()))
    }

    /// 获取消息列表（优化版：支持时间戳分页，避免子查询）
    pub async fn get_messages(
        &self,
        user_id: &str,
        friend_id: &str,
        before_time: Option<chrono::DateTime<Utc>>,
        limit: i32,
    ) -> Result<(Vec<MessageResponse>, bool), AppError> {
        // 1. 验证好友关系
        if !verify_friendship(&self.db, user_id, friend_id).await? {
            return Err(AppError::BadRequest("不是好友关系".to_string()));
        }

        // 2. 生成会话UUID
        let conversation_uuid = generate_conversation_uuid(user_id, friend_id);

        // 3. 查询消息（使用复合索引 idx-friend-messages-conv-time 优化）
        let messages: Vec<Message> = if let Some(before) = before_time {
            // 分页查询：直接使用时间戳，避免子查询
            sqlx::query_as(
                r#"
                SELECT "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                       "message-content", "message-type", "file-uuid", "file-url", "file-size", "send-time",
                       "is-deleted-by-sender", "is-deleted-by-receiver"
                FROM "friend-messages"
                WHERE "conversation-uuid" = $1
                  AND "send-time" < $2
                  AND (
                      ("sender-id" = $3 AND "is-deleted-by-sender" = false) OR
                      ("receiver-id" = $3 AND "is-deleted-by-receiver" = false)
                  )
                ORDER BY "send-time" DESC
                LIMIT $4
                "#,
            )
            .bind(&conversation_uuid)
            .bind(before)
            .bind(user_id)
            .bind(limit + 1)
            .fetch_all(&self.db)
            .await
        } else {
            // 查询最新消息
            sqlx::query_as(
                r#"
                SELECT "message-uuid", "conversation-uuid", "sender-id", "receiver-id",
                       "message-content", "message-type", "file-uuid", "file-url", "file-size", "send-time",
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
        .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

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
    pub async fn delete_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AppError> {
        // 1. 查询消息
        let message: Option<(String, String)> = sqlx::query_as(
            r#"SELECT "sender-id", "receiver-id" FROM "friend-messages" WHERE "message-uuid" = $1"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

        let (sender_id, receiver_id) = message.ok_or(AppError::BadRequest("消息不存在".to_string()))?;

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
            return Err(AppError::Forbidden);
        }
        .map_err(|e| AppError::Database(format!("删除消息失败: {}", e)))?;

        Ok(())
    }

    /// 撤回消息（双方都标记为已删除）
    pub async fn recall_message(&self, user_id: &str, message_uuid: &str) -> Result<(), AppError> {
        // 1. 查询消息
        let message: Option<(String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT "sender-id", "send-time" FROM "friend-messages" WHERE "message-uuid" = $1"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

        let (sender_id, send_time) = message.ok_or(AppError::BadRequest("消息不存在".to_string()))?;

        // 2. 只有发送者可以撤回
        if user_id != sender_id {
            return Err(AppError::Forbidden);
        }

        // 3. 检查是否超过撤回时间窗口（使用配置的撤回时限）
        let now = Utc::now();
        let duration = now.signed_duration_since(send_time);
        let config = message_config();
        if duration.num_seconds() > config.recall_window as i64 {
            let window_minutes = config.recall_window / 60;
            return Err(AppError::BadRequest(format!(
                "消息发送超过{}分钟，无法撤回",
                window_minutes
            )));
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
        .map_err(|e| AppError::Database(format!("撤回消息失败: {}", e)))?;

        Ok(())
    }
}

