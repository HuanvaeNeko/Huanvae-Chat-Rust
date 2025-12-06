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
    /// 
    /// 注意：未读计数更新已统一由 NotificationService 处理
    /// 参见 websocket/services/notification_service.rs 中的 notify_group_message
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
        .map_err(|e| AppError::Database(format!("发送群消息失败: {}", e)))?;

        // 未读计数更新已移至 NotificationService.notify_group_message
        // 通过 UnreadService.batch_increment_group_unread 统一处理
        // 这样避免了重复的 SQL 逻辑，保持单一数据源

        Ok(SendMessageResponse {
            message_uuid: message_uuid.to_string(),
            send_time: now.to_rfc3339(),
        })
    }

    /// 获取群消息列表
    /// 获取群消息列表（优化版：JOIN一次性获取用户信息 + 时间戳分页）
    pub async fn get_messages(
        &self,
        group_id: &Uuid,
        user_id: &str,
        before_time: Option<chrono::DateTime<Utc>>,
        limit: i32,
    ) -> Result<GroupMessagesResponse, AppError> {
        // 使用 JOIN 一次性获取消息和发送者信息，消除 N+1 问题
        // 使用复合索引 idx-group-messages-group-time 优化查询
        let messages: Vec<GroupMessageWithSender> = if let Some(before) = before_time {
            sqlx::query_as(
                r#"SELECT 
                    m."message-uuid", m."group-id", m."sender-id", m."message-content",
                    m."message-type", m."file-uuid", m."file-url", m."file-size",
                    m."reply-to", m."send-time", m."is-recalled", m."recalled-at", m."recalled-by",
                    u."user-nickname" as sender_nickname,
                    u."user-avatar-url" as sender_avatar_url
                   FROM "group-messages" m
                   LEFT JOIN "users" u ON u."user-id" = m."sender-id"
                   LEFT JOIN "group-message-deletions" d 
                     ON d."message-uuid" = m."message-uuid" AND d."user-id" = $1
                   WHERE m."group-id" = $2 
                     AND m."send-time" < $3
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
                r#"SELECT 
                    m."message-uuid", m."group-id", m."sender-id", m."message-content",
                    m."message-type", m."file-uuid", m."file-url", m."file-size",
                    m."reply-to", m."send-time", m."is-recalled", m."recalled-at", m."recalled-by",
                    u."user-nickname" as sender_nickname,
                    u."user-avatar-url" as sender_avatar_url
                   FROM "group-messages" m
                   LEFT JOIN "users" u ON u."user-id" = m."sender-id"
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
        .map_err(|e| AppError::Database(format!("查询群消息失败: {}", e)))?;

        let has_more = messages.len() > limit as usize;
        let result: Vec<GroupMessageInfo> = messages
            .into_iter()
            .take(limit as usize)
            .map(GroupMessageInfo::from)
            .collect();

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
        .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

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
        .map_err(|e| AppError::Database(format!("删除消息失败: {}", e)))?;

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
        .map_err(|e| AppError::Database(format!("查询消息失败: {}", e)))?;

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
        .map_err(|e| AppError::Database(format!("撤回消息失败: {}", e)))?;

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
        .map_err(|e| AppError::Database(format!("标记已读失败: {}", e)))?;

        Ok(())
    }
}

