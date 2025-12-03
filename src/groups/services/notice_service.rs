//! 群公告服务

use crate::common::AppError;
use crate::groups::models::*;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// 公告服务
#[derive(Clone)]
pub struct NoticeService {
    db: PgPool,
}

impl NoticeService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 发布公告
    pub async fn publish_notice(
        &self,
        group_id: &Uuid,
        publisher_id: &str,
        title: Option<&str>,
        content: &str,
        is_pinned: bool,
    ) -> Result<PublishNoticeResponse, AppError> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO "group-notices"
               ("id", "group-id", "title", "content", "publisher-id", "published-at", "is-pinned")
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(id)
        .bind(group_id)
        .bind(title)
        .bind(content)
        .bind(publisher_id)
        .bind(now)
        .bind(is_pinned)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("发布公告失败: {}", e);
            AppError::Internal
        })?;

        Ok(PublishNoticeResponse {
            id: id.to_string(),
            published_at: now.to_rfc3339(),
        })
    }

    /// 获取公告列表
    pub async fn get_notices(&self, group_id: &Uuid) -> Result<Vec<NoticeInfo>, AppError> {
        let notices: Vec<GroupNotice> = sqlx::query_as(
            r#"SELECT * FROM "group-notices" 
               WHERE "group-id" = $1 AND "is-active" = true
               ORDER BY "is-pinned" DESC, "published-at" DESC"#,
        )
        .bind(group_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询公告列表失败: {}", e);
            AppError::Internal
        })?;

        // 获取发布者昵称
        let mut result = Vec::new();
        for notice in notices {
            let publisher_nickname: Option<(Option<String>,)> = sqlx::query_as(
                r#"SELECT "user-nickname" FROM "users" WHERE "user-id" = $1"#,
            )
            .bind(&notice.publisher_id)
            .fetch_optional(&self.db)
            .await
            .ok()
            .flatten();

            let mut info = NoticeInfo::from(notice);
            info.publisher_nickname = publisher_nickname.and_then(|(n,)| n);
            result.push(info);
        }

        Ok(result)
    }

    /// 更新公告
    pub async fn update_notice(
        &self,
        notice_id: &Uuid,
        title: Option<&str>,
        content: Option<&str>,
        is_pinned: Option<bool>,
    ) -> Result<(), AppError> {
        // 如果没有任何更新字段，直接返回
        if title.is_none() && content.is_none() && is_pinned.is_none() {
            return Ok(());
        }

        // 分别处理不同情况
        match (title, content, is_pinned) {
            (Some(t), Some(c), Some(p)) => {
                sqlx::query(r#"UPDATE "group-notices" SET "title" = $1, "content" = $2, "is-pinned" = $3 WHERE "id" = $4 AND "is-active" = true"#)
                    .bind(t).bind(c).bind(p).bind(notice_id)
                    .execute(&self.db).await
            }
            (Some(t), Some(c), None) => {
                sqlx::query(r#"UPDATE "group-notices" SET "title" = $1, "content" = $2 WHERE "id" = $3 AND "is-active" = true"#)
                    .bind(t).bind(c).bind(notice_id)
                    .execute(&self.db).await
            }
            (Some(t), None, Some(p)) => {
                sqlx::query(r#"UPDATE "group-notices" SET "title" = $1, "is-pinned" = $2 WHERE "id" = $3 AND "is-active" = true"#)
                    .bind(t).bind(p).bind(notice_id)
                    .execute(&self.db).await
            }
            (None, Some(c), Some(p)) => {
                sqlx::query(r#"UPDATE "group-notices" SET "content" = $1, "is-pinned" = $2 WHERE "id" = $3 AND "is-active" = true"#)
                    .bind(c).bind(p).bind(notice_id)
                    .execute(&self.db).await
            }
            (Some(t), None, None) => {
                sqlx::query(r#"UPDATE "group-notices" SET "title" = $1 WHERE "id" = $2 AND "is-active" = true"#)
                    .bind(t).bind(notice_id)
                    .execute(&self.db).await
            }
            (None, Some(c), None) => {
                sqlx::query(r#"UPDATE "group-notices" SET "content" = $1 WHERE "id" = $2 AND "is-active" = true"#)
                    .bind(c).bind(notice_id)
                    .execute(&self.db).await
            }
            (None, None, Some(p)) => {
                sqlx::query(r#"UPDATE "group-notices" SET "is-pinned" = $1 WHERE "id" = $2 AND "is-active" = true"#)
                    .bind(p).bind(notice_id)
                    .execute(&self.db).await
            }
            (None, None, None) => return Ok(()),
        }
        .map_err(|e| {
            tracing::error!("更新公告失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 删除公告（软删除）
    pub async fn delete_notice(
        &self,
        notice_id: &Uuid,
        deleted_by: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"UPDATE "group-notices" SET 
               "is-active" = false,
               "deleted-at" = $1,
               "deleted-by" = $2
               WHERE "id" = $3 AND "is-active" = true"#,
        )
        .bind(now)
        .bind(deleted_by)
        .bind(notice_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("删除公告失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("公告不存在".to_string()));
        }

        Ok(())
    }
}

