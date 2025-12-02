use crate::friends::services::FriendsService;
use sqlx::PgPool;

/// 好友模块 Handler 状态
#[derive(Clone)]
pub struct FriendsState {
    pub service: FriendsService,
    pub db: PgPool,
}

impl FriendsState {
    pub fn new(db: PgPool) -> Self {
        Self {
            service: FriendsService::new(db.clone()),
            db,
        }
    }
}