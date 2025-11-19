use crate::friends::services::FriendsState;
use sqlx::PgPool;

#[derive(Clone)]
pub struct FriendsRouterState {
    pub friends_state: FriendsState,
    pub db: PgPool,
}