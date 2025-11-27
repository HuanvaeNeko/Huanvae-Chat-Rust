use sqlx::PgPool;

/// 消息服务状态
#[derive(Clone)]
pub struct MessagesState {
    pub db: PgPool,
}

impl MessagesState {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

