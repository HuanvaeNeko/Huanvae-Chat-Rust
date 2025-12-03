-- ========================================
-- HuanVae Chat 核心表结构
-- 创建时间: 2025-12-03
-- 包含: users, groups 表及其基础索引、触发器、视图
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
    
    -- 群聊字段 (待迁移到独立表)
    "user-joined-group-chats" TEXT DEFAULT '',
    
    -- AI对话数据 (待迁移到 JSONB)
    "user-ai-conversation-data" TEXT DEFAULT '',
    
    -- 黑名单检查控制字段（智能性能优化）
    "need-blacklist-check" BOOLEAN DEFAULT false,
    "blacklist-check-expires-at" TIMESTAMP,
    
    -- 时间戳
    "created-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 群聊数据表 (groups)
-- ========================================
CREATE TABLE IF NOT EXISTS "groups" (
    -- 基础字段
    "group-id" TEXT PRIMARY KEY,
    "group-name" TEXT NOT NULL,
    
    -- 成员管理字段 (待迁移到独立表)
    "group-members" TEXT DEFAULT '',
    "group-administrators" TEXT DEFAULT '',
    "group-owner" TEXT DEFAULT '',
    
    -- 群聊内容字段 (待迁移到独立表)
    "group-chat-data" TEXT DEFAULT '',
    "group-muted-members" TEXT DEFAULT '',
    "group-notice" TEXT DEFAULT '',
    "group-chat-file-data-and-url" TEXT DEFAULT '',
    
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

-- 群聊表索引
CREATE INDEX IF NOT EXISTS "idx-groups-name" ON "groups"("group-name");
CREATE INDEX IF NOT EXISTS "idx-groups-created-at" ON "groups"("created-at" DESC);

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

-- 群聊表更新触发器
DROP TRIGGER IF EXISTS "trigger-update-groups-timestamp" ON "groups";
CREATE TRIGGER "trigger-update-groups-timestamp"
BEFORE UPDATE ON "groups"
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

-- 群聊基础信息视图
CREATE OR REPLACE VIEW "view-groups-basic" AS
SELECT 
    "group-id",
    "group-name",
    "created-at",
    "updated-at"
FROM "groups";

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '核心表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. users  - 用户数据表';
\echo '  2. groups - 群聊数据表';
\echo '';
\echo '创建的索引:';
\echo '  用户表: 6个索引';
\echo '  群聊表: 2个索引';
\echo '';
\echo '创建的触发器:';
\echo '  - trigger-update-users-timestamp';
\echo '  - trigger-update-groups-timestamp';
\echo '';
\echo '创建的视图:';
\echo '  - view-users-basic';
\echo '  - view-groups-basic';
\echo '';
\echo '========================================';

