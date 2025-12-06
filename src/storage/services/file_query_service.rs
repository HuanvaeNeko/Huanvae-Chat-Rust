//! 文件查询服务
//!
//! 负责处理文件列表查询

use sqlx::PgPool;
use sqlx::Row;

use crate::common::AppError;
use crate::storage::models::*;

/// 文件查询服务
pub struct FileQueryService {
    db: PgPool,
    api_base_url: String,
}

impl FileQueryService {
    pub fn new(db: PgPool, api_base_url: String) -> Self {
        Self { db, api_base_url }
    }

    /// 查询用户文件列表（支持分页、过滤、排序）
    /// 只返回用户个人文件（storage_location = 'user_files'），不包含好友和群聊文件
    pub async fn list_user_files(
        &self,
        user_id: &str,
        page: i32,
        limit: i32,
        sort_by: String,
        sort_order: String,
    ) -> Result<FileListResponse, AppError> {
        // 1. 参数验证
        let page = page.max(1);
        let limit = limit.clamp(1, 100);
        let offset = (page - 1) * limit;
        
        // 2. 确定排序字段
        let sort_column = match sort_by.as_str() {
            "file_size" => r#""file-size""#,
            _ => r#""created-at""#,
        };
        let sort_dir = if sort_order == "asc" { "ASC" } else { "DESC" };
        
        // 3. 查询总数
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT m."uuid") as count
            FROM "file-uuid-mapping" m
            INNER JOIN "file-access-permissions" p ON m."uuid" = p."file-uuid"
            INNER JOIN "file-records" r ON r."file-uuid" = m."uuid"
            WHERE p."user-id" = $1 
              AND p."revoked-at" IS NULL
              AND r."owner-id" = $1
              AND r."storage-location" = 'user_files'
            "#
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;
        
        // 4. 构建查询SQL
        let query_sql = format!(
            r#"
            SELECT * FROM (
                SELECT DISTINCT ON (m."uuid")
                    m."uuid", m."physical-file-key", m."file-size", 
                    m."content-type", m."preview-support", m."created-at"
                FROM "file-uuid-mapping" m
                INNER JOIN "file-access-permissions" p ON m."uuid" = p."file-uuid"
                INNER JOIN "file-records" r ON r."file-uuid" = m."uuid"
                WHERE p."user-id" = $1 
                  AND p."revoked-at" IS NULL
                  AND r."owner-id" = $1
                  AND r."storage-location" = 'user_files'
                ORDER BY m."uuid"
            ) AS unique_files
            ORDER BY {} {}
            LIMIT $2 OFFSET $3
            "#,
            sort_column, sort_dir
        );
        
        // 5. 执行查询
        let rows = sqlx::query(&query_sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?;
        
        // 6. 转换为响应格式
        let files: Vec<FileItem> = rows
            .into_iter()
            .map(|row| {
                let uuid: String = row.try_get("uuid").unwrap_or_default();
                let physical_key: String = row.try_get("physical-file-key").unwrap_or_default();
                let filename = Self::extract_filename_from_key(&physical_key);
                
                FileItem {
                    file_uuid: uuid.clone(),
                    filename,
                    file_size: row.try_get("file-size").unwrap_or(0),
                    content_type: row.try_get("content-type").unwrap_or_default(),
                    preview_support: row.try_get("preview-support").unwrap_or_default(),
                    created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created-at")
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    file_url: format!("{}/api/storage/file/{}", self.api_base_url, uuid),
                }
            })
            .collect();
        
        // 7. 计算分页信息
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        let has_more = page < total_pages;
        
        Ok(FileListResponse {
            files,
            total,
            page,
            page_size: limit,
            total_pages,
            has_more,
        })
    }
    
    /// 从file_key中提取原始文件名
    fn extract_filename_from_key(file_key: &str) -> String {
        file_key
            .split('/')
            .last()
            .and_then(|s| {
                let parts: Vec<&str> = s.splitn(3, '_').collect();
                parts.get(2).map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

