//! 通知推送服务
//!
//! 处理消息通知的推送和已读同步

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::common::AppError;
use crate::config::websocket_config;
use crate::websocket::models::{truncate_preview, ServerMessage, SourceType, SystemNotificationType};
use crate::websocket::services::{ConnectionManager, UnreadService};

/// 通知推送服务
#[derive(Clone)]
pub struct NotificationService {
    db: PgPool,
    connection_manager: Arc<ConnectionManager>,
    unread_service: UnreadService,
}

impl NotificationService {
    /// 创建通知服务
    pub fn new(db: PgPool, connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            unread_service: UnreadService::new(db.clone()),
            db,
            connection_manager,
        }
    }

    /// 好友消息通知
    ///
    /// 当用户发送好友消息时调用
    pub async fn notify_friend_message(
        &self,
        sender_id: &str,
        sender_nickname: &str,
        receiver_id: &str,
        message_uuid: &str,
        message_content: &str,
        message_type: &str,
        send_time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        // 1. 更新接收方未读计数
        self.unread_service
            .increment_friend_unread(
                receiver_id,
                sender_id,
                message_uuid,
                &truncate_preview(message_content, 50),
                message_type,
                send_time,
            )
            .await?;

        // 2. 如果接收方在线，推送通知
        if self.connection_manager.is_online(receiver_id) {
            let notification = ServerMessage::NewMessage {
                source_type: SourceType::Friend,
                source_id: sender_id.to_string(),
                message_uuid: message_uuid.to_string(),
                sender_id: sender_id.to_string(),
                sender_nickname: sender_nickname.to_string(),
                preview: truncate_preview(message_content, 50),
                message_type: message_type.to_string(),
                timestamp: send_time,
            };

            self.connection_manager.send_to_user(receiver_id, &notification);

            debug!(
                sender_id = %sender_id,
                receiver_id = %receiver_id,
                "Friend message notification sent"
            );
        }

        Ok(())
    }

    /// 群聊消息通知
    ///
    /// 当用户发送群消息时调用
    pub async fn notify_group_message(
        &self,
        group_id: &Uuid,
        _group_name: &str,
        sender_id: &str,
        sender_nickname: &str,
        message_uuid: &Uuid,
        message_content: &str,
        message_type: &str,
        send_time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        // 1. 获取群成员列表（排除发送者）
        let members = self.get_group_members_except(group_id, sender_id).await?;

        // 2. 批量更新未读计数
        self.unread_service
            .batch_increment_group_unread(
                group_id,
                sender_id,
                message_uuid,
                &truncate_preview(message_content, 50),
                message_type,
                send_time,
            )
            .await?;

        // 3. 向在线成员推送通知
        let notification = ServerMessage::NewMessage {
            source_type: SourceType::Group,
            source_id: group_id.to_string(),
            message_uuid: message_uuid.to_string(),
            sender_id: sender_id.to_string(),
            sender_nickname: sender_nickname.to_string(),
            preview: truncate_preview(message_content, 50),
            message_type: message_type.to_string(),
            timestamp: send_time,
        };

        let online_members: Vec<String> = members
            .iter()
            .filter(|m| self.connection_manager.is_online(m))
            .cloned()
            .collect();

        if !online_members.is_empty() {
            self.connection_manager.send_to_users(&online_members, &notification);

            debug!(
                group_id = %group_id,
                online_count = online_members.len(),
                "Group message notification sent"
            );
        }

        Ok(())
    }

    /// 消息撤回通知
    pub async fn notify_message_recalled(
        &self,
        source_type: SourceType,
        source_id: &str,
        message_uuid: &str,
        recalled_by: &str,
        target_user_ids: &[String],
    ) -> Result<(), AppError> {
        let notification = ServerMessage::MessageRecalled {
            source_type,
            source_id: source_id.to_string(),
            message_uuid: message_uuid.to_string(),
            recalled_by: recalled_by.to_string(),
        };

        self.connection_manager.send_to_users(target_user_ids, &notification);

        info!(
            source_type = %source_type,
            source_id = %source_id,
            message_uuid = %message_uuid,
            "Message recall notification sent"
        );

        Ok(())
    }

    /// 已读同步通知
    ///
    /// 当用户标记已读时，通知对方（仅当功能开启时）
    pub async fn notify_read_sync(
        &self,
        source_type: SourceType,
        source_id: &str,
        reader_id: &str,
        target_user_id: &str,
    ) -> Result<(), AppError> {
        // 检查是否开启已读回执功能
        if !websocket_config().enable_read_receipt {
            debug!("Read receipt disabled, skipping read sync notification");
            return Ok(());
        }

        let notification = ServerMessage::ReadSync {
            source_type,
            source_id: source_id.to_string(),
            reader_id: reader_id.to_string(),
            read_at: Utc::now(),
        };

        self.connection_manager.send_to_user(target_user_id, &notification);

        debug!(
            reader_id = %reader_id,
            target_user_id = %target_user_id,
            "Read sync notification sent"
        );

        Ok(())
    }

    /// 系统通知
    pub async fn notify_system(
        &self,
        user_id: &str,
        notification_type: SystemNotificationType,
        data: serde_json::Value,
    ) -> Result<(), AppError> {
        let notification = ServerMessage::SystemNotification {
            notification_type,
            data,
        };

        self.connection_manager.send_to_user(user_id, &notification);

        Ok(())
    }

    /// 获取群成员列表（排除指定用户）
    async fn get_group_members_except(
        &self,
        group_id: &Uuid,
        exclude_user_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_scalar!(
            r#"
            SELECT "user-id"
            FROM "group-members"
            WHERE "group-id" = $1 
              AND "user-id" != $2
              AND "status" = 'active'
            "#,
            group_id,
            exclude_user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            error!("Failed to get group members: {}", e);
            AppError::Database(e.to_string())
        })?;

        Ok(rows)
    }

    /// 获取连接管理器（供外部使用）
    pub fn connection_manager(&self) -> &Arc<ConnectionManager> {
        &self.connection_manager
    }

    /// 获取未读服务（供外部使用）
    pub fn unread_service(&self) -> &UnreadService {
        &self.unread_service
    }
}

