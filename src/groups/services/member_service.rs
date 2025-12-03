//! 群成员管理服务

use crate::common::AppError;
use crate::groups::models::*;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// 成员管理服务
#[derive(Clone)]
pub struct MemberService {
    db: PgPool,
}

impl MemberService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 获取用户在群中的角色信息
    pub async fn get_member_role(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<Option<MemberBrief>, AppError> {
        let member: Option<MemberBrief> = sqlx::query_as(
            r#"SELECT "user-id", "role", "status", "muted-until" 
               FROM "group-members" 
               WHERE "group-id" = $1 AND "user-id" = $2"#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询成员角色失败: {}", e);
            AppError::Internal
        })?;

        Ok(member)
    }

    /// 验证用户是否为活跃群成员
    pub async fn verify_active_member(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<bool, AppError> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"SELECT "status" FROM "group-members" 
               WHERE "group-id" = $1 AND "user-id" = $2"#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("验证成员状态失败: {}", e);
            AppError::Internal
        })?;

        Ok(result.map(|(s,)| s == "active").unwrap_or(false))
    }

    /// 验证用户是否为群主或管理员
    pub async fn verify_admin_or_owner(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<bool, AppError> {
        let result: Option<(String, String)> = sqlx::query_as(
            r#"SELECT "role", "status" FROM "group-members" 
               WHERE "group-id" = $1 AND "user-id" = $2"#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("验证管理员权限失败: {}", e);
            AppError::Internal
        })?;

        Ok(result
            .map(|(role, status)| status == "active" && (role == "owner" || role == "admin"))
            .unwrap_or(false))
    }

    /// 验证用户是否为群主
    pub async fn verify_owner(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<bool, AppError> {
        let result: Option<(String, String)> = sqlx::query_as(
            r#"SELECT "role", "status" FROM "group-members" 
               WHERE "group-id" = $1 AND "user-id" = $2"#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("验证群主权限失败: {}", e);
            AppError::Internal
        })?;

        Ok(result
            .map(|(role, status)| status == "active" && role == "owner")
            .unwrap_or(false))
    }

    /// 获取群成员列表
    pub async fn get_members(
        &self,
        group_id: &Uuid,
    ) -> Result<Vec<MemberInfo>, AppError> {
        let rows: Vec<(String, Option<String>, Option<String>, String, Option<String>, chrono::DateTime<Utc>, String, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
            r#"SELECT 
                gm."user-id", u."user-nickname", u."user-avatar-url",
                gm."role", gm."group-nickname", gm."joined-at", gm."join-method", gm."muted-until"
               FROM "group-members" gm
               LEFT JOIN "users" u ON u."user-id" = gm."user-id"
               WHERE gm."group-id" = $1 AND gm."status" = 'active'
               ORDER BY 
                 CASE gm."role" 
                   WHEN 'owner' THEN 1 
                   WHEN 'admin' THEN 2 
                   ELSE 3 
                 END,
                 gm."joined-at" ASC"#,
        )
        .bind(group_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询群成员列表失败: {}", e);
            AppError::Internal
        })?;

        let members = rows
            .into_iter()
            .map(|(user_id, user_nickname, user_avatar_url, role, group_nickname, joined_at, join_method, muted_until)| {
                MemberInfo {
                    user_id,
                    user_nickname,
                    user_avatar_url,
                    role,
                    group_nickname,
                    joined_at: joined_at.to_rfc3339(),
                    join_method,
                    muted_until: muted_until.map(|t| t.to_rfc3339()),
                }
            })
            .collect();

        Ok(members)
    }

    /// 添加成员到群
    pub async fn add_member(
        &self,
        group_id: &Uuid,
        user_id: &str,
        join_method: &str,
        invited_by: Option<&str>,
        approved_by: Option<&str>,
        invite_code_id: Option<&Uuid>,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO "group-members"
               ("group-id", "user-id", "role", "join-method", "invited-by", "approved-by", "invite-code-id", "joined-at", "status")
               VALUES ($1, $2, 'member', $3, $4, $5, $6, $7, 'active')
               ON CONFLICT ("group-id", "user-id") 
               DO UPDATE SET 
                 "role" = 'member',
                 "join-method" = $3,
                 "invited-by" = $4,
                 "approved-by" = $5,
                 "invite-code-id" = $6,
                 "joined-at" = $7,
                 "status" = 'active',
                 "left-at" = NULL,
                 "left-reason" = NULL,
                 "removed-by" = NULL,
                 "removed-reason" = NULL"#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(join_method)
        .bind(invited_by)
        .bind(approved_by)
        .bind(invite_code_id)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("添加群成员失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 成员主动退出群聊
    pub async fn leave_group(
        &self,
        group_id: &Uuid,
        user_id: &str,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let leave_reason = reason.unwrap_or("主动退出");

        let result = sqlx::query(
            r#"UPDATE "group-members" SET 
               "status" = 'left',
               "left-at" = $1,
               "left-reason" = $2
               WHERE "group-id" = $3 AND "user-id" = $4 AND "status" = 'active'"#,
        )
        .bind(now)
        .bind(leave_reason)
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("退出群聊失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("您不是该群成员".to_string()));
        }

        // 删除未读消息记录
        sqlx::query(r#"DELETE FROM "group-unread-messages" WHERE "group-id" = $1 AND "user-id" = $2"#)
            .bind(group_id)
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("删除未读消息记录失败: {}", e);
                AppError::Internal
            })?;

        Ok(())
    }

    /// 移除成员（踢人）
    pub async fn remove_member(
        &self,
        group_id: &Uuid,
        user_id: &str,
        removed_by: &str,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let remove_reason = reason.unwrap_or("被管理员移出群聊");

        let result = sqlx::query(
            r#"UPDATE "group-members" SET 
               "status" = 'removed',
               "left-at" = $1,
               "removed-by" = $2,
               "removed-reason" = $3
               WHERE "group-id" = $4 AND "user-id" = $5 AND "status" = 'active'"#,
        )
        .bind(now)
        .bind(removed_by)
        .bind(remove_reason)
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("移除成员失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("该用户不是群成员".to_string()));
        }

        // 删除未读消息记录
        sqlx::query(r#"DELETE FROM "group-unread-messages" WHERE "group-id" = $1 AND "user-id" = $2"#)
            .bind(group_id)
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("删除未读消息记录失败: {}", e);
                AppError::Internal
            })?;

        Ok(())
    }

    /// 转让群主
    /// 
    /// 群主可以将群主头衔移交给任何人：
    /// - 如果新群主是管理员，则从管理员变为群主
    /// - 如果新群主是普通成员，则直接变为群主
    /// - 旧群主变为普通成员
    pub async fn transfer_owner(
        &self,
        group_id: &Uuid,
        old_owner_id: &str,
        new_owner_id: &str,
    ) -> Result<(), AppError> {
        // 开始事务
        let mut tx = self.db.begin().await.map_err(|e| {
            tracing::error!("开始事务失败: {}", e);
            AppError::Internal
        })?;

        // 将新群主设为 owner
        let result = sqlx::query(
            r#"UPDATE "group-members" SET "role" = 'owner'
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active'"#,
        )
        .bind(group_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("设置新群主失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("目标用户不是该群成员".to_string()));
        }

        // 将旧群主设为普通成员
        sqlx::query(
            r#"UPDATE "group-members" SET "role" = 'member'
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active'"#,
        )
        .bind(group_id)
        .bind(old_owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("更新旧群主角色失败: {}", e);
            AppError::Internal
        })?;

        // 提交事务
        tx.commit().await.map_err(|e| {
            tracing::error!("提交事务失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 设置管理员
    pub async fn set_admin(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "group-members" SET "role" = 'admin'
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active' AND "role" = 'member'"#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("设置管理员失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("该用户不是普通成员或不存在".to_string()));
        }

        Ok(())
    }

    /// 取消管理员
    pub async fn remove_admin(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "group-members" SET "role" = 'member'
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active' AND "role" = 'admin'"#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("取消管理员失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("该用户不是管理员或不存在".to_string()));
        }

        Ok(())
    }

    /// 禁言成员
    pub async fn mute_member(
        &self,
        group_id: &Uuid,
        user_id: &str,
        muted_by: &str,
        duration_minutes: i64,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let muted_until = if duration_minutes <= 0 {
            // 永久禁言（设置为100年后）
            Utc::now() + Duration::days(365 * 100)
        } else {
            Utc::now() + Duration::minutes(duration_minutes)
        };

        let result = sqlx::query(
            r#"UPDATE "group-members" SET 
               "muted-until" = $1,
               "muted-by" = $2,
               "muted-reason" = $3
               WHERE "group-id" = $4 AND "user-id" = $5 AND "status" = 'active'"#,
        )
        .bind(muted_until)
        .bind(muted_by)
        .bind(reason)
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("禁言成员失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("该用户不是群成员".to_string()));
        }

        Ok(())
    }

    /// 解除禁言
    pub async fn unmute_member(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"UPDATE "group-members" SET 
               "muted-until" = NULL,
               "muted-by" = NULL,
               "muted-reason" = NULL
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active'"#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("解除禁言失败: {}", e);
            AppError::Internal
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("该用户不是群成员".to_string()));
        }

        Ok(())
    }

    /// 检查用户是否被禁言
    pub async fn is_muted(
        &self,
        group_id: &Uuid,
        user_id: &str,
    ) -> Result<bool, AppError> {
        let result: Option<(Option<chrono::DateTime<Utc>>,)> = sqlx::query_as(
            r#"SELECT "muted-until" FROM "group-members" 
               WHERE "group-id" = $1 AND "user-id" = $2 AND "status" = 'active'"#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("检查禁言状态失败: {}", e);
            AppError::Internal
        })?;

        match result {
            Some((Some(muted_until),)) => Ok(muted_until > Utc::now()),
            _ => Ok(false),
        }
    }
}

