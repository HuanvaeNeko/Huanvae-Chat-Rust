//! 未读消息服务
//!
//! 管理好友和群聊的未读消息计数

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, error};
use uuid::Uuid;

use crate::common::AppError;
use crate::websocket::models::{FriendUnread, GroupUnread, UnreadSummary};

/// 好友未读消息查询结果行
#[derive(Debug)]
struct FriendUnreadRow {
    friend_id: String,
    friend_nickname: Option<String>,
    friend_avatar: Option<String>,
    unread_count: Option<i32>,
    last_message_content: Option<String>,
    last_message_type: Option<String>,
    last_message_time: Option<DateTime<Utc>>,
}

/// 群聊未读消息查询结果行
#[derive(Debug)]
struct GroupUnreadRow {
    group_id: Uuid,
    group_name: String,
    group_avatar: Option<String>,
    unread_count: Option<i32>,
    last_message_content: Option<String>,
    last_message_type: Option<String>,
    last_sender_nickname: Option<String>,
    last_message_time: Option<DateTime<Utc>>,
}

/// 未读消息服务
#[derive(Clone)]
pub struct UnreadService {
    db: PgPool,
}

impl UnreadService {
    /// 创建未读消息服务
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 获取用户的未读消息摘要
    pub async fn get_unread_summary(&self, user_id: &str) -> Result<UnreadSummary, AppError> {
        let friend_unreads = self.get_friend_unreads(user_id).await?;
        let group_unreads = self.get_group_unreads(user_id).await?;

        let total_count: i32 = friend_unreads.iter().map(|f| f.unread_count).sum::<i32>()
            + group_unreads.iter().map(|g| g.unread_count).sum::<i32>();

        Ok(UnreadSummary {
            friend_unreads,
            group_unreads,
            total_count,
        })
    }

    /// 获取好友未读消息列表
    async fn get_friend_unreads(&self, user_id: &str) -> Result<Vec<FriendUnread>, AppError> {
        let rows = sqlx::query_as!(
            FriendUnreadRow,
            r#"
            SELECT 
                u."friend-id" as friend_id,
                usr."user-nickname" as friend_nickname,
                COALESCE(usr."user-avatar-url", '') as friend_avatar,
                u."unread-count" as unread_count,
                COALESCE(u."last-message-content", '') as last_message_content,
                COALESCE(u."last-message-type", 'text') as last_message_type,
                u."last-message-time" as last_message_time
            FROM "friend-unread-messages" u
            LEFT JOIN "users" usr ON usr."user-id" = u."friend-id"
            WHERE u."user-id" = $1 AND u."unread-count" > 0
            ORDER BY u."last-message-time" DESC NULLS LAST
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to get friend unreads: {}", e);
            AppError::Database(e.to_string())
        })?;

        Ok(rows
            .into_iter()
            .map(|row| FriendUnread {
                friend_id: row.friend_id,
                friend_nickname: row.friend_nickname.unwrap_or_default(),
                friend_avatar: row.friend_avatar.unwrap_or_default(),
                unread_count: row.unread_count.unwrap_or(0),
                last_message_preview: row.last_message_content.unwrap_or_default(),
                last_message_type: row.last_message_type.unwrap_or_else(|| "text".to_string()),
                last_message_time: row.last_message_time,
            })
            .collect())
    }

    /// 获取群聊未读消息列表
    async fn get_group_unreads(&self, user_id: &str) -> Result<Vec<GroupUnread>, AppError> {
        let rows = sqlx::query_as!(
            GroupUnreadRow,
            r#"
            SELECT 
                u."group-id" as group_id,
                g."group-name" as group_name,
                COALESCE(g."group-avatar-url", '') as group_avatar,
                u."unread-count" as unread_count,
                COALESCE(u."last-message-content", '') as last_message_content,
                COALESCE(u."last-message-type", 'text') as last_message_type,
                COALESCE(sender."user-nickname", u."last-sender-id", '') as last_sender_nickname,
                u."last-message-time" as last_message_time
            FROM "group-unread-messages" u
            JOIN "groups" g ON g."group-id" = u."group-id"
            LEFT JOIN "users" sender ON sender."user-id" = u."last-sender-id"
            WHERE u."user-id" = $1 AND u."unread-count" > 0 AND g."status" = 'active'
            ORDER BY u."last-message-time" DESC NULLS LAST
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to get group unreads: {}", e);
            AppError::Database(e.to_string())
        })?;

        Ok(rows
            .into_iter()
            .map(|row| GroupUnread {
                group_id: row.group_id.to_string(),
                group_name: row.group_name,
                group_avatar: row.group_avatar.unwrap_or_default(),
                unread_count: row.unread_count.unwrap_or(0),
                last_message_preview: row.last_message_content.unwrap_or_default(),
                last_message_type: row.last_message_type.unwrap_or_else(|| "text".to_string()),
                last_sender_nickname: row.last_sender_nickname.unwrap_or_default(),
                last_message_time: row.last_message_time,
            })
            .collect())
    }

    /// 标记好友消息已读
    pub async fn mark_friend_read(&self, user_id: &str, friend_id: &str) -> Result<(), AppError> {
        let conversation_uuid = crate::common::generate_conversation_uuid(user_id, friend_id);

        sqlx::query!(
            r#"
            UPDATE "friend-unread-messages"
            SET "unread-count" = 0,
                "updated-at" = CURRENT_TIMESTAMP
            WHERE "user-id" = $1 AND "conversation-uuid" = $2
            "#,
            user_id,
            conversation_uuid
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to mark friend read: {}", e);
            AppError::Database(e.to_string())
        })?;

        debug!(
            user_id = %user_id,
            friend_id = %friend_id,
            "Marked friend messages as read"
        );

        Ok(())
    }

    /// 标记群聊消息已读
    pub async fn mark_group_read(&self, user_id: &str, group_id: &Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE "group-unread-messages"
            SET "unread-count" = 0,
                "updated-at" = CURRENT_TIMESTAMP
            WHERE "user-id" = $1 AND "group-id" = $2
            "#,
            user_id,
            group_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to mark group read: {}", e);
            AppError::Database(e.to_string())
        })?;

        debug!(
            user_id = %user_id,
            group_id = %group_id,
            "Marked group messages as read"
        );

        Ok(())
    }

    /// 增加好友未读计数
    ///
    /// 当收到好友消息时调用
    pub async fn increment_friend_unread(
        &self,
        user_id: &str,
        friend_id: &str,
        message_uuid: &str,
        message_content: &str,
        message_type: &str,
        send_time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let conversation_uuid = crate::common::generate_conversation_uuid(user_id, friend_id);
        let unread_id = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO "friend-unread-messages" (
                "unread-id", "user-id", "conversation-uuid", "friend-id",
                "unread-count", "last-message-uuid", "last-message-content",
                "last-message-type", "last-message-time"
            )
            VALUES ($1, $2, $3, $4, 1, $5, $6, $7, $8)
            ON CONFLICT ("user-id", "conversation-uuid")
            DO UPDATE SET
                "unread-count" = "friend-unread-messages"."unread-count" + 1,
                "last-message-uuid" = $5,
                "last-message-content" = $6,
                "last-message-type" = $7,
                "last-message-time" = $8,
                "updated-at" = CURRENT_TIMESTAMP
            "#,
            unread_id,
            user_id,
            conversation_uuid,
            friend_id,
            message_uuid,
            message_content,
            message_type,
            send_time
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to increment friend unread: {}", e);
            AppError::Database(e.to_string())
        })?;

        Ok(())
    }

    /// 增加群聊未读计数
    ///
    /// 当收到群消息时调用
    pub async fn increment_group_unread(
        &self,
        user_id: &str,
        group_id: &Uuid,
        sender_id: &str,
        message_uuid: &Uuid,
        message_content: &str,
        message_type: &str,
        send_time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO "group-unread-messages" (
                "id", "user-id", "group-id",
                "unread-count", "last-message-uuid", "last-message-content",
                "last-message-type", "last-message-time", "last-sender-id"
            )
            VALUES (gen_random_uuid(), $1, $2, 1, $3, $4, $5, $6, $7)
            ON CONFLICT ("user-id", "group-id")
            DO UPDATE SET
                "unread-count" = "group-unread-messages"."unread-count" + 1,
                "last-message-uuid" = $3,
                "last-message-content" = $4,
                "last-message-type" = $5,
                "last-message-time" = $6,
                "last-sender-id" = $7,
                "updated-at" = CURRENT_TIMESTAMP
            "#,
            user_id,
            group_id,
            message_uuid,
            message_content,
            message_type,
            send_time,
            sender_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to increment group unread: {}", e);
            AppError::Database(e.to_string())
        })?;

        Ok(())
    }

    /// 批量增加群成员未读计数
    ///
    /// 当发送群消息时，为所有群成员（排除发送者）增加未读计数
    pub async fn batch_increment_group_unread(
        &self,
        group_id: &Uuid,
        sender_id: &str,
        message_uuid: &Uuid,
        message_content: &str,
        message_type: &str,
        send_time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        // 使用单条 SQL 批量更新所有群成员的未读计数
        sqlx::query!(
            r#"
            INSERT INTO "group-unread-messages" (
                "id", "user-id", "group-id",
                "unread-count", "last-message-uuid", "last-message-content",
                "last-message-type", "last-message-time", "last-sender-id"
            )
            SELECT 
                gen_random_uuid(),
                gm."user-id",
                $1,
                1,
                $2, $3, $4, $5, $6
            FROM "group-members" gm
            WHERE gm."group-id" = $1 
              AND gm."user-id" != $6
              AND gm."status" = 'active'
            ON CONFLICT ("user-id", "group-id")
            DO UPDATE SET
                "unread-count" = "group-unread-messages"."unread-count" + 1,
                "last-message-uuid" = $2,
                "last-message-content" = $3,
                "last-message-type" = $4,
                "last-message-time" = $5,
                "last-sender-id" = $6,
                "updated-at" = CURRENT_TIMESTAMP
            "#,
            group_id,
            message_uuid,
            message_content,
            message_type,
            send_time,
            sender_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to batch increment group unread: {}", e);
            AppError::Database(e.to_string())
        })?;

        debug!(
            group_id = %group_id,
            sender_id = %sender_id,
            "Batch incremented group unread for all members"
        );

        Ok(())
    }

    /// 获取好友未读计数
    pub async fn get_friend_unread_count(
        &self,
        user_id: &str,
        friend_id: &str,
    ) -> Result<i32, AppError> {
        let conversation_uuid = crate::common::generate_conversation_uuid(user_id, friend_id);

        let result = sqlx::query_scalar!(
            r#"
            SELECT "unread-count"
            FROM "friend-unread-messages"
            WHERE "user-id" = $1 AND "conversation-uuid" = $2
            "#,
            user_id,
            conversation_uuid
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.flatten().unwrap_or(0))
    }

    /// 获取群聊未读计数
    pub async fn get_group_unread_count(
        &self,
        user_id: &str,
        group_id: &Uuid,
    ) -> Result<i32, AppError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT "unread-count"
            FROM "group-unread-messages"
            WHERE "user-id" = $1 AND "group-id" = $2
            "#,
            user_id,
            group_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.flatten().unwrap_or(0))
    }

    /// 获取用户总未读数
    pub async fn get_total_unread_count(&self, user_id: &str) -> Result<i32, AppError> {
        let friend_count = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(SUM("unread-count"), 0) as "count!"
            FROM "friend-unread-messages"
            WHERE "user-id" = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let group_count = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(SUM("unread-count"), 0) as "count!"
            FROM "group-unread-messages"
            WHERE "user-id" = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok((friend_count + group_count) as i32)
    }
}

