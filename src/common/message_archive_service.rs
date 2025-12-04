//! 消息归档服务
//!
//! 提供消息归档和缓存清理功能

use crate::common::AppError;
use sqlx::PgPool;

/// 消息归档服务
#[derive(Clone)]
pub struct MessageArchiveService {
    db: PgPool,
}

impl MessageArchiveService {
    /// 创建新的消息归档服务
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 归档指定天数前的消息
    /// 返回 (好友消息归档数, 群消息归档数)
    pub async fn archive_old_messages(&self, archive_days: i32) -> Result<(i64, i64), AppError> {
        let result: (i64, i64) = sqlx::query_as(
            r#"SELECT * FROM archive_old_messages($1)"#,
        )
        .bind(archive_days)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("消息归档失败: {}", e)))?;

        Ok(result)
    }

    /// 清理过期的消息缓存
    /// 返回清理的缓存数量
    pub async fn cleanup_expired_cache(&self) -> Result<i32, AppError> {
        let result: (i32,) = sqlx::query_as(
            r#"SELECT cleanup_expired_message_cache()"#,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("缓存清理失败: {}", e)))?;

        Ok(result.0)
    }

    /// 获取当前活跃表的消息统计
    /// 返回 (好友消息数, 群消息数)
    pub async fn get_active_message_counts(&self) -> Result<(i64, i64), AppError> {
        let friend_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "friend-messages""#,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("查询失败: {}", e)))?;

        let group_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "group-messages""#,
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("查询失败: {}", e)))?;

        Ok((friend_count.0, group_count.0))
    }

    /// 获取归档表的消息统计
    /// 返回 (好友消息归档数, 群消息归档数)
    pub async fn get_archive_message_counts(&self) -> Result<(i64, i64), AppError> {
        let friend_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "friend-messages-archive""#,
        )
        .fetch_one(&self.db)
        .await
        .unwrap_or((0,));

        let group_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "group-messages-archive""#,
        )
        .fetch_one(&self.db)
        .await
        .unwrap_or((0,));

        Ok((friend_count.0, group_count.0))
    }
}

