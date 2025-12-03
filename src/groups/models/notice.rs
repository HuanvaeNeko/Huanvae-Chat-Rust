//! 群公告模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 群公告数据库模型
#[derive(Debug, Clone, FromRow)]
pub struct GroupNotice {
    pub id: Uuid,
    #[sqlx(rename = "group-id")]
    pub group_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    #[sqlx(rename = "publisher-id")]
    pub publisher_id: String,
    #[sqlx(rename = "published-at")]
    pub published_at: DateTime<Utc>,
    #[sqlx(rename = "is-pinned")]
    pub is_pinned: bool,
    #[sqlx(rename = "is-active")]
    pub is_active: bool,
    #[sqlx(rename = "deleted-at")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "deleted-by")]
    pub deleted_by: Option<String>,
    #[sqlx(rename = "updated-at")]
    pub updated_at: DateTime<Utc>,
}

/// 群公告信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeInfo {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub publisher_id: String,
    pub publisher_nickname: Option<String>,
    pub published_at: String,
    pub is_pinned: bool,
    pub updated_at: String,
}

impl From<GroupNotice> for NoticeInfo {
    fn from(n: GroupNotice) -> Self {
        Self {
            id: n.id.to_string(),
            title: n.title,
            content: n.content,
            publisher_id: n.publisher_id,
            publisher_nickname: None,
            published_at: n.published_at.to_rfc3339(),
            is_pinned: n.is_pinned,
            updated_at: n.updated_at.to_rfc3339(),
        }
    }
}

/// 群公告列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct NoticeListResponse {
    pub notices: Vec<NoticeInfo>,
}

/// 发布公告响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishNoticeResponse {
    pub id: String,
    pub published_at: String,
}

