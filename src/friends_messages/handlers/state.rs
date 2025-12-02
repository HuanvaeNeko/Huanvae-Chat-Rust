use crate::friends_messages::services::MessageService;
use sqlx::PgPool;

/// 消息模块 Handler 状态
#[derive(Clone)]
pub struct MessagesState {
    pub service: MessageService,
    pub db: PgPool,
}

impl MessagesState {
    pub fn new(db: PgPool) -> Self {
        Self {
            service: MessageService::new(db.clone()),
            db,
        }
    }
}

