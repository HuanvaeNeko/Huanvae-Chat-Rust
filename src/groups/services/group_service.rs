//! 群聊核心服务

use crate::common::AppError;
use crate::groups::models::*;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// 群聊服务
#[derive(Clone)]
pub struct GroupService {
    db: PgPool,
}

impl GroupService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 创建群聊
    pub async fn create_group(
        &self,
        creator_id: &str,
        group_name: &str,
        group_description: Option<&str>,
        join_mode: Option<&str>,
    ) -> Result<CreateGroupResponse, AppError> {
        let group_id = Uuid::now_v7();
        let join_mode = join_mode.unwrap_or("approval_required");
        let now = Utc::now();

        // 开始事务
        let mut tx = self.db.begin().await.map_err(|e| {
            tracing::error!("开始事务失败: {}", e);
            AppError::Internal
        })?;

        // 创建群聊
        sqlx::query(
            r#"INSERT INTO "groups" 
               ("group-id", "group-name", "group-description", "creator-id", "join-mode", "created-at", "member-count")
               VALUES ($1, $2, $3, $4, $5, $6, 1)"#,
        )
        .bind(group_id)
        .bind(group_name)
        .bind(group_description)
        .bind(creator_id)
        .bind(join_mode)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("创建群聊失败: {}", e);
            AppError::Internal
        })?;

        // 添加创建者为群主
        sqlx::query(
            r#"INSERT INTO "group-members"
               ("group-id", "user-id", "role", "join-method", "joined-at", "status")
               VALUES ($1, $2, 'owner', 'create', $3, 'active')"#,
        )
        .bind(group_id)
        .bind(creator_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("添加群主失败: {}", e);
            AppError::Internal
        })?;

        // 提交事务
        tx.commit().await.map_err(|e| {
            tracing::error!("提交事务失败: {}", e);
            AppError::Internal
        })?;

        Ok(CreateGroupResponse {
            group_id: group_id.to_string(),
            group_name: group_name.to_string(),
            created_at: now.to_rfc3339(),
        })
    }

    /// 获取群聊信息
    pub async fn get_group_info(&self, group_id: &Uuid) -> Result<GroupInfo, AppError> {
        let group: Group = sqlx::query_as(
            r#"SELECT * FROM "groups" WHERE "group-id" = $1"#,
        )
        .bind(group_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询群聊失败: {}", e);
            AppError::Internal
        })?
        .ok_or_else(|| AppError::NotFound("群聊不存在".to_string()))?;

        Ok(GroupInfo::from(group))
    }

    /// 更新群聊信息
    pub async fn update_group_info(
        &self,
        group_id: &Uuid,
        group_name: Option<&str>,
        group_description: Option<&str>,
    ) -> Result<(), AppError> {
        if group_name.is_none() && group_description.is_none() {
            return Ok(());
        }

        let mut query = String::from(r#"UPDATE "groups" SET "#);
        let mut params: Vec<String> = Vec::new();
        let mut param_index = 1;

        if let Some(name) = group_name {
            params.push(format!(r#""group-name" = ${}"#, param_index));
            param_index += 1;
            let _ = name;
        }
        if let Some(desc) = group_description {
            params.push(format!(r#""group-description" = ${}"#, param_index));
            param_index += 1;
            let _ = desc;
        }

        query.push_str(&params.join(", "));
        query.push_str(&format!(r#" WHERE "group-id" = ${}"#, param_index));

        // 使用简化的更新方式
        if let Some(name) = group_name {
            if group_description.is_some() {
                sqlx::query(r#"UPDATE "groups" SET "group-name" = $1, "group-description" = $2 WHERE "group-id" = $3"#)
                    .bind(name)
                    .bind(group_description)
                    .bind(group_id)
                    .execute(&self.db)
                    .await
            } else {
                sqlx::query(r#"UPDATE "groups" SET "group-name" = $1 WHERE "group-id" = $2"#)
                    .bind(name)
                    .bind(group_id)
                    .execute(&self.db)
                    .await
            }
        } else if let Some(desc) = group_description {
            sqlx::query(r#"UPDATE "groups" SET "group-description" = $1 WHERE "group-id" = $2"#)
                .bind(desc)
                .bind(group_id)
                .execute(&self.db)
                .await
        } else {
            return Ok(());
        }
        .map_err(|e| {
            tracing::error!("更新群聊信息失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 修改入群模式（仅群主）
    pub async fn update_join_mode(
        &self,
        group_id: &Uuid,
        join_mode: &str,
    ) -> Result<(), AppError> {
        // 验证入群模式有效性
        if JoinMode::from_str(join_mode).is_none() {
            return Err(AppError::BadRequest("无效的入群模式".to_string()));
        }

        sqlx::query(r#"UPDATE "groups" SET "join-mode" = $1 WHERE "group-id" = $2"#)
            .bind(join_mode)
            .bind(group_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("更新入群模式失败: {}", e);
                AppError::Internal
            })?;

        Ok(())
    }

    /// 解散群聊（软删除）
    pub async fn disband_group(
        &self,
        group_id: &Uuid,
        owner_id: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        // 开始事务
        let mut tx = self.db.begin().await.map_err(|e| {
            tracing::error!("开始事务失败: {}", e);
            AppError::Internal
        })?;

        // 更新群状态
        sqlx::query(
            r#"UPDATE "groups" SET 
               "status" = 'disbanded', 
               "disbanded-at" = $1, 
               "disbanded-by" = $2
               WHERE "group-id" = $3"#,
        )
        .bind(now)
        .bind(owner_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("更新群状态失败: {}", e);
            AppError::Internal
        })?;

        // 更新所有成员状态
        sqlx::query(
            r#"UPDATE "group-members" SET 
               "status" = 'left', 
               "left-at" = $1, 
               "left-reason" = '群聊已解散'
               WHERE "group-id" = $2 AND "status" = 'active'"#,
        )
        .bind(now)
        .bind(group_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("更新成员状态失败: {}", e);
            AppError::Internal
        })?;

        // 撤销所有邀请码
        sqlx::query(
            r#"UPDATE "group-invite-codes" SET 
               "status" = 'revoked', 
               "revoked-at" = $1, 
               "revoked-by" = $2
               WHERE "group-id" = $3 AND "status" = 'active'"#,
        )
        .bind(now)
        .bind(owner_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("撤销邀请码失败: {}", e);
            AppError::Internal
        })?;

        // 取消所有待处理的入群申请
        sqlx::query(
            r#"UPDATE "group-join-requests" SET 
               "status" = 'cancelled'
               WHERE "group-id" = $1 AND "status" = 'pending'"#,
        )
        .bind(group_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("取消入群申请失败: {}", e);
            AppError::Internal
        })?;

        // 删除未读消息记录
        sqlx::query(r#"DELETE FROM "group-unread-messages" WHERE "group-id" = $1"#)
            .bind(group_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("删除未读消息记录失败: {}", e);
                AppError::Internal
            })?;

        // 提交事务
        tx.commit().await.map_err(|e| {
            tracing::error!("提交事务失败: {}", e);
            AppError::Internal
        })?;

        Ok(())
    }

    /// 获取用户加入的群聊列表
    pub async fn get_user_groups(&self, user_id: &str) -> Result<Vec<GroupListItem>, AppError> {
        let rows: Vec<(Uuid, String, Option<String>, String, Option<i32>, Option<String>, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
            r#"SELECT 
                g."group-id", g."group-name", g."group-avatar-url", 
                gm."role",
                gu."unread-count", gu."last-message-content", gu."last-message-time"
               FROM "group-members" gm
               JOIN "groups" g ON g."group-id" = gm."group-id"
               LEFT JOIN "group-unread-messages" gu ON gu."group-id" = gm."group-id" AND gu."user-id" = gm."user-id"
               WHERE gm."user-id" = $1 AND gm."status" = 'active' AND g."status" = 'active'
               ORDER BY COALESCE(gu."last-message-time", gm."joined-at") DESC"#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询用户群聊列表失败: {}", e);
            AppError::Internal
        })?;

        let groups = rows
            .into_iter()
            .map(|(group_id, group_name, group_avatar_url, role, unread_count, last_message_content, last_message_time)| {
                GroupListItem {
                    group_id: group_id.to_string(),
                    group_name,
                    group_avatar_url,
                    role,
                    unread_count,
                    last_message_content,
                    last_message_time: last_message_time.map(|t| t.to_rfc3339()),
                }
            })
            .collect();

        Ok(groups)
    }

    /// 更新群头像URL
    pub async fn update_group_avatar(
        &self,
        group_id: &Uuid,
        avatar_url: &str,
    ) -> Result<(), AppError> {
        sqlx::query(r#"UPDATE "groups" SET "group-avatar-url" = $1 WHERE "group-id" = $2"#)
            .bind(avatar_url)
            .bind(group_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("更新群头像失败: {}", e);
                AppError::Internal
            })?;

        Ok(())
    }

    /// 验证群是否存在且为活跃状态
    pub async fn verify_group_active(&self, group_id: &Uuid) -> Result<bool, AppError> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"SELECT "status" FROM "groups" WHERE "group-id" = $1"#,
        )
        .bind(group_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| {
            tracing::error!("查询群状态失败: {}", e);
            AppError::Internal
        })?;

        match result {
            Some((status,)) => Ok(status == "active"),
            None => Err(AppError::NotFound("群聊不存在".to_string())),
        }
    }

    /// 增加成员计数
    pub async fn increment_member_count(&self, group_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(r#"UPDATE "groups" SET "member-count" = "member-count" + 1 WHERE "group-id" = $1"#)
            .bind(group_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("增加成员计数失败: {}", e);
                AppError::Internal
            })?;
        Ok(())
    }

    /// 减少成员计数
    pub async fn decrement_member_count(&self, group_id: &Uuid) -> Result<(), AppError> {
        sqlx::query(r#"UPDATE "groups" SET "member-count" = "member-count" - 1 WHERE "group-id" = $1 AND "member-count" > 0"#)
            .bind(group_id)
            .execute(&self.db)
            .await
            .map_err(|e| {
                tracing::error!("减少成员计数失败: {}", e);
                AppError::Internal
            })?;
        Ok(())
    }
}

