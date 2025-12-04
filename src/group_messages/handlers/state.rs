//! 群消息模块状态

use crate::group_messages::services::GroupMessageService;
use crate::groups::services::MemberService;
use crate::websocket::services::NotificationService;
use sqlx::PgPool;

/// 群消息模块状态
#[derive(Clone)]
pub struct GroupMessagesState {
    pub message_service: GroupMessageService,
    pub member_service: MemberService,
    pub db: PgPool,
    pub notification_service: Option<NotificationService>,
}

impl GroupMessagesState {
    pub fn new(db: PgPool) -> Self {
        Self {
            message_service: GroupMessageService::new(db.clone()),
            member_service: MemberService::new(db.clone()),
            db,
            notification_service: None,
        }
    }

    /// 带通知服务的构造函数
    pub fn with_notification(db: PgPool, notification_service: NotificationService) -> Self {
        Self {
            message_service: GroupMessageService::new(db.clone()),
            member_service: MemberService::new(db.clone()),
            db,
            notification_service: Some(notification_service),
        }
    }
}

