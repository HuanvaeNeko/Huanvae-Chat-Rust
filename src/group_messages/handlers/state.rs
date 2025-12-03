//! 群消息模块状态

use crate::group_messages::services::GroupMessageService;
use crate::groups::services::MemberService;
use sqlx::PgPool;

/// 群消息模块状态
#[derive(Clone)]
pub struct GroupMessagesState {
    pub message_service: GroupMessageService,
    pub member_service: MemberService,
    pub db: PgPool,
}

impl GroupMessagesState {
    pub fn new(db: PgPool) -> Self {
        Self {
            message_service: GroupMessageService::new(db.clone()),
            member_service: MemberService::new(db.clone()),
            db,
        }
    }
}

