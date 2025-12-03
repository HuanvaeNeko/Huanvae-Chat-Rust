//! 群消息服务

use crate::common::AppError;
use crate::config::message_config;
use crate::group_messages::models::*;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// 群消息服务
#[derive(Clone)]
pub struct GroupMessageService {
    db: PgPool,
}

impl GroupMessageService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 发送群消息
    pub async fn send_message(
        &self,
        group_id: &Uuid,
        sender_id: &str,
        message_content: &str,
        message_type: &str,
        file_uuid: Option<&str>,
        file_url: Option<&str>,
        file_size: Option<i64>,
        reply_to: Option<&Uuid>,
    ) -> Result<SendMessageResponse, AppError> {
        let message_uuid = Uuid::now_v7();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO "group-messages"
               ("message-uuid", "group-id", "sender-id", "message-content", "message-type", 
                "file-uuid", "file-url", "file-size", "reply-to", "send-time")
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        )
        .bind(message_uuid)
        .bind(group_id)
        .bind(sender_id)
        .bind(message_content)
        .bind(message_type)
        .bind(file_uuid)
        .bind(file_url)
        .bind(file_size)
        .bind(reply_to)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("发送群消息失败: {}", e);
            AppError::Internal
        })?;

        // 更新所有群成员的未读消息计数
        sqlx::query(
            r#"INSERT INTO "group-unread-messages" 
               ("user-id", "group-id", "unread-count", "last-message-uuid", "last-message-content", 
                "last-message-type", "last-message-time", "last-sender-id")
               SELECT gm."user-id", $1, 1, $2, $3, $4, $5, $6
               FROM "group-members" gm
               WHERE gm."group-id" = $1 AND gm."status" = 'active' AND gm."user-id" != $6
               ON CONFLICT ("user-id", "group-id") DO UPDATE SET
                 "unread-count" = "group-unread-messages"."unread-count" + 1,
                 "last-message-uuid" = $2,
                 "last-message-content" = $3,
                 "last-message-type" = $4,
                 "last-message-time" = $5,
                 "last-sender-id" = $6,
                 "updated-at" = CURRENT_TIMESTAMP"#,
        )
        .bind(group_id)
        .bind(message_uuid)
        .bind(message_content)
        .bind(message_type)
        .bind(now)
        .bind(sender_id)
        .execute(&self.db)
        .await
        .ok(); // 未读计数更新失败不影响消息发送

        Ok(SendMessageResponse {
            message_uuid: message_uuid.to_string(),
            send_time: now.to_rfc3339(),
        })
    }

    /// 获取群消息列表
    pub async fn get_messages(
        &self,
        group_id: &Uuid,
        user_id: &str,
        before_uuid: Option<&Uuid>,
        limit: i32,
    ) -> Result<GroupMessagesResponse, AppError> {
        let messages: Vec<GroupMessage> = if let Some(before) = before_uuid {
            sqlx::query_as(
                r#"SELECT m.* FROM "group-messages" m
                   LEFT JOIN "group-message-deletions" d 
                     ON d."message-uuid" = m."message-uuid" AND d."user-id" = $1
                   WHERE m."group-id" = $2 
                     AND m."send-time" < (SELECT "send-time" FROM "group-messages" WHERE "message-uuid" = $3)
                     AND d."id" IS NULL
                   ORDER BY m."send-time" DESC
                   LIMIT $4"#,
            )
            .bind(user_id)
            .bind(group_id)
            .bind(before)
            .bind(limit + 1)
            .fetch_all(&self.db)
            .await
        } else {
            sqlx::query_as(
                r#"SELECT m.* FROM "group-messages" m
                   LEFT JOIN "group-message-deletions" d 
                     ON d."message-uuid" = m."message-uuid" AND d."user-id" = $1
                   WHERE m."group-id" = $2 AND d."id" IS NULL
                   ORDER BY m."send-time" DESC
                   LIMIT $3"#,
            )
            .bind(user_id)
            .bind(group_id)
            .bind(limit + 1)
            .fetch_all(&self.db)
            .await
        }
        .map_err(|e| {
            tracing::error!("查询群消息失败: {}", e);
            AppError::Internal
        })?;

        let has_more = messages.len() > limit as usize;
        let messages: Vec<GroupMessage> = messages.into_iter().take(limit as usize).collect();

        // 获取发送者信息
        let mut result = Vec::new();
        for msg in messages {
            let sender_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                r#"SELECT "user-nickname", "user-avatar-url" FROM "users" WHERE "user-id" = $1"#,
            )
            .bind(&msg.sender_id)
            .fetch_optional(&self.db)
            .await
            .ok()
            .flatten();

            let mut info = GroupMessageInfo::from(msg);
            if let Some((nickname, avatar)) = sender_info {
                info.sender_nickname = nickname;
                info.sender_avatar_url = avatar;
            }
            result.push(info);
        }

        Ok(GroupMessagesResponse {
            messages: result,
            has_more,
        })
    }

    /// 删除群消息（个人删除）
    pub async fn delete_message(
        &self,
        message_uuid: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        // 检查消息是否存在
        let exists: Option<(Uuid,)> = sqlx::query_as(
            r#"SELECT "message-uuid" FROM "group-messages" WHERE "message-uuid" = $1"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::Internal)?;

        if exists.is_none() {
            return Err(AppError::BadRequest("消息不存在".to_string()));
        }

        // 插入删除记录
        sqlx::query(
            r#"INSERT INTO "group-message-deletions" ("message-uuid", "user-id")
               VALUES ($1, $2)
               ON CONFLICT ("message-uuid", "user-id") DO NOTHING"#,
        )
        .bind(message_uuid)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("删除消息失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 撤回群消息
    /// 
    /// - 发送者：只能撤回2分钟内的消息
    /// - 群主/管理员：可以撤回任意消息
    pub async fn recall_message(
        &self,
        message_uuid: &Uuid,
        user_id: &str,
        is_admin_or_owner: bool,
    ) -> Result<(), AppError> {
        // 查询消息
        let message: Option<(String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT "sender-id", "send-time" FROM "group-messages" 
               WHERE "message-uuid" = $1 AND "is-recalled" = false"#,
        )
        .bind(message_uuid)
        .fetch_optional(&self.db)
        .await
        .map_err(|_| AppError::Internal)?;

        let (sender_id, send_time) = message.ok_or_else(|| {
            AppError::BadRequest("消息不存在或已撤回".to_string())
        })?;

        // 权限检查
        if !is_admin_or_owner {
            // 普通成员只能撤回自己的消息
            if sender_id != user_id {
                return Err(AppError::Forbidden);
            }

            // 检查时间限制
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
        }

        // 执行撤回
        let now = Utc::now();
        sqlx::query(
            r#"UPDATE "group-messages" SET 
               "is-recalled" = true,
               "recalled-at" = $1,
               "recalled-by" = $2
               WHERE "message-uuid" = $3"#,
        )
        .bind(now)
        .bind(user_id)
        .bind(message_uuid)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("撤回消息失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 标记已读（清除未读计数）
    pub async fn mark_as_read(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE "group-unread-messages" SET "unread-count" = 0, "updated-at" = CURRENT_TIMESTAMP
               WHERE "group-id" = $1 AND "user-id" = $2"#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("标记已读失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }
}

