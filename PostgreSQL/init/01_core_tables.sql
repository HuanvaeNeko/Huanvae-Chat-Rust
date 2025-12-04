-- ========================================
-- HuanVae Chat 核心表结构
-- 创建时间: 2025-12-03
-- 包含: users 表及其基础索引、触发器、视图
-- 注意: groups 表已迁移至 06_group_system.sql
-- ========================================

-- ========================================
-- 用户数据表 (users)
-- ========================================
CREATE TABLE IF NOT EXISTS "users" (
    -- 基础字段
    "user-id" TEXT PRIMARY KEY,
    "user-nickname" TEXT NOT NULL,
    "user-password" TEXT NOT NULL,
    "user-email" TEXT,
    "user-signature" TEXT DEFAULT '',
    "user-avatar-url" TEXT DEFAULT '',
    "admin" TEXT DEFAULT 'false',
    
    -- 黑名单检查控制字段（智能性能优化）
    "need-blacklist-check" BOOLEAN DEFAULT false,
    "blacklist-check-expires-at" TIMESTAMP,
    
    -- 时间戳
    "created-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 创建索引
-- ========================================

-- 用户表索引
CREATE INDEX IF NOT EXISTS "idx-users-email" ON "users"("user-email");
CREATE INDEX IF NOT EXISTS "idx-users-nickname" ON "users"("user-nickname");
CREATE INDEX IF NOT EXISTS "idx-users-admin" ON "users"("admin");
CREATE INDEX IF NOT EXISTS "idx-users-created-at" ON "users"("created-at" DESC);
CREATE INDEX IF NOT EXISTS "idx-users-blacklist-check" ON "users"("need-blacklist-check") WHERE "need-blacklist-check" = true;
CREATE INDEX IF NOT EXISTS "idx-users-avatar-url" ON "users"("user-avatar-url");

-- ========================================
-- 创建触发器函数
-- ========================================
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW."updated-at" = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 用户表更新触发器
DROP TRIGGER IF EXISTS "trigger-update-users-timestamp" ON "users";
CREATE TRIGGER "trigger-update-users-timestamp"
BEFORE UPDATE ON "users"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- ========================================
-- 创建视图
-- ========================================

-- 用户基础信息视图
CREATE OR REPLACE VIEW "view-users-basic" AS
SELECT 
    "user-id",
    "user-nickname",
    "user-email",
    "admin",
    "created-at",
    "updated-at"
FROM "users";

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '核心表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. users  - 用户数据表';
\echo '';
\echo '创建的索引:';
\echo '  用户表: 6个索引';
\echo '';
\echo '创建的触发器:';
\echo '  - trigger-update-users-timestamp';
\echo '';
\echo '创建的视图:';
\echo '  - view-users-basic';
\echo '';
\echo '注意: groups 表已迁移至 06_group_system.sql';
\echo '========================================';

