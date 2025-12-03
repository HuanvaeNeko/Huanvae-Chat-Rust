//! 群聊模块状态

use crate::groups::services::{GroupService, MemberService, InviteCodeService, NoticeService};
use sqlx::PgPool;

/// 群聊模块状态
#[derive(Clone)]
pub struct GroupsState {
    pub group_service: GroupService,
    pub member_service: MemberService,
    pub invite_code_service: InviteCodeService,
    pub notice_service: NoticeService,
    pub db: PgPool,
}

impl GroupsState {
    pub fn new(db: PgPool) -> Self {
        Self {
            group_service: GroupService::new(db.clone()),
            member_service: MemberService::new(db.clone()),
            invite_code_service: InviteCodeService::new(db.clone()),
            notice_service: NoticeService::new(db.clone()),
            db,
        }
    }
}

