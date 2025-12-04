use crate::friends_messages::services::MessageService;
use crate::websocket::services::NotificationService;
use sqlx::PgPool;

/// 消息模块 Handler 状态
#[derive(Clone)]
pub struct MessagesState {
    pub service: MessageService,
    pub db: PgPool,
    pub notification_service: Option<NotificationService>,
}

impl MessagesState {
    pub fn new(db: PgPool) -> Self {
        Self {
            service: MessageService::new(db.clone()),
            db,
            notification_service: None,
        }
    }

    /// 带通知服务的构造函数
    pub fn with_notification(db: PgPool, notification_service: NotificationService) -> Self {
        Self {
            service: MessageService::new(db.clone()),
            db,
            notification_service: Some(notification_service),
        }
    }
}

