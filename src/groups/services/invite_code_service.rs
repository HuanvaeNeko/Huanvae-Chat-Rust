//! 邀请码服务

use crate::common::AppError;
use crate::groups::models::*;
use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

/// 邀请码服务
#[derive(Clone)]
pub struct InviteCodeService {
    db: PgPool,
}

impl InviteCodeService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 生成随机邀请码（8位 base62）
    fn generate_code() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 创建邀请码
    pub async fn create_invite_code(
        &self,
        group_id: &Uuid,
        creator_id: &str,
        creator_role: &str,
        max_uses: Option<i32>,
        expires_in_hours: Option<i32>,
    ) -> Result<CreateInviteCodeResponse, AppError> {
        // 根据角色确定邀请码类型
        let code_type = if creator_role == "owner" || creator_role == "admin" {
            "direct"
        } else {
            "normal"
        };

        let code = Self::generate_code();
        let id = Uuid::now_v7();
        let now = Utc::now();
        let expires_at = expires_in_hours.map(|h| now + Duration::hours(h as i64));

        sqlx::query(
            r#"INSERT INTO "group-invite-codes"
               ("id", "group-id", "code", "code-type", "creator-id", "creator-role", "max-uses", "expires-at", "created-at")
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(id)
        .bind(group_id)
        .bind(&code)
        .bind(code_type)
        .bind(creator_id)
        .bind(creator_role)
        .bind(max_uses)
        .bind(expires_at)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("创建邀请码失败: {}", e)))?;

        Ok(CreateInviteCodeResponse {
            id: id.to_string(),
            code,
            code_type: code_type.to_string(),
            expires_at: expires_at.map(|t| t.to_rfc3339()),
        })
    }

    /// 获取群的邀请码列表
    pub async fn get_invite_codes(
        &self,
        group_id: &Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<InviteCodeInfo>, AppError> {
        let codes: Vec<InviteCode> = if is_admin {
            // 管理员可以看到所有邀请码
            sqlx::query_as(
                r#"SELECT * FROM "group-invite-codes" 
                   WHERE "group-id" = $1 AND "status" = 'active'
                   ORDER BY "created-at" DESC"#,
            )
            .bind(group_id)
            .fetch_all(&self.db)
            .await
        } else {
            // 普通成员只能看到自己创建的
            sqlx::query_as(
                r#"SELECT * FROM "group-invite-codes" 
                   WHERE "group-id" = $1 AND "creator-id" = $2 AND "status" = 'active'
                   ORDER BY "created-at" DESC"#,
            )
            .bind(group_id)
            .bind(user_id)
            .fetch_all(&self.db)
            .await
        }
        .map_err(|e| AppError::Database(format!("查询邀请码列表失败: {}", e)))?;

        Ok(codes.into_iter().map(InviteCodeInfo::from).collect())
    }

    /// 撤销邀请码
    pub async fn revoke_invite_code(
        &self,
        code_id: &Uuid,
        revoked_by: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"UPDATE "group-invite-codes" SET 
               "status" = 'revoked',
               "revoked-at" = $1,
               "revoked-by" = $2
               WHERE "id" = $3 AND "status" = 'active'"#,
        )
        .bind(now)
        .bind(revoked_by)
        .bind(code_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("撤销邀请码失败: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::BadRequest("邀请码不存在或已失效".to_string()));
        }

        Ok(())
    }

    /// 验证并使用邀请码
    pub async fn validate_and_use_code(
        &self,
        code: &str,
    ) -> Result<(Uuid, InviteCode), AppError> {
        let now = Utc::now();

        // 查询邀请码
        let invite_code: InviteCode = sqlx::query_as(
            r#"SELECT * FROM "group-invite-codes" WHERE "code" = $1 AND "status" = 'active'"#,
        )
        .bind(code)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("查询邀请码失败: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("邀请码无效或已失效".to_string()))?;

        // 检查是否过期
        if let Some(expires_at) = invite_code.expires_at {
            if expires_at < now {
                // 更新状态为过期
                sqlx::query(r#"UPDATE "group-invite-codes" SET "status" = 'expired' WHERE "id" = $1"#)
                    .bind(invite_code.id)
                    .execute(&self.db)
                    .await
                    .ok();
                return Err(AppError::BadRequest("邀请码已过期".to_string()));
            }
        }

        // 检查使用次数
        if let Some(max_uses) = invite_code.max_uses {
            if invite_code.used_count >= max_uses {
                // 更新状态为用尽
                sqlx::query(r#"UPDATE "group-invite-codes" SET "status" = 'exhausted' WHERE "id" = $1"#)
                    .bind(invite_code.id)
                    .execute(&self.db)
                    .await
                    .ok();
                return Err(AppError::BadRequest("邀请码使用次数已达上限".to_string()));
            }
        }

        // 增加使用次数
        sqlx::query(
            r#"UPDATE "group-invite-codes" SET "used-count" = "used-count" + 1 WHERE "id" = $1"#,
        )
        .bind(invite_code.id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database(format!("更新邀请码使用次数失败: {}", e)))?;

        // 检查是否达到上限，如果是则更新状态
        if let Some(max_uses) = invite_code.max_uses {
            if invite_code.used_count + 1 >= max_uses {
                sqlx::query(r#"UPDATE "group-invite-codes" SET "status" = 'exhausted' WHERE "id" = $1"#)
                    .bind(invite_code.id)
                    .execute(&self.db)
                    .await
                    .ok();
            }
        }

        Ok((invite_code.group_id, invite_code))
    }
}

