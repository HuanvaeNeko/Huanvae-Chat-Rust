use sqlx::PgPool;

/// 消息处理器状态
#[derive(Clone)]
pub struct MessagesState {
    pub db: PgPool,
}

impl MessagesState {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

