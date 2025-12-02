-- ========================================
-- HuanVae Chat 数据库初始化脚本
-- 基于 data.md 设计文档
-- 创建时间: 2025-01-15
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
    
    -- 好友关系已移至独立表: friendships, friend_requests (见 07_friendships_tables.sql)
    
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
    
    -- 成员管理字段
    -- 存储格式: member-id:user-001,member-nickname:张三,join-time:2025-01-10,member-role:member;...
    "group-members" TEXT DEFAULT '',
    
    -- 存储格式: admin-id:user-002,admin-nickname:李四,promote-time:2025-01-12,admin-permissions:manage;...
    "group-administrators" TEXT DEFAULT '',
    
    -- 存储格式: owner-id:user-001,owner-nickname:张三,group-create-time:2025-01-10,become-owner-time:2025-01-10
    "group-owner" TEXT DEFAULT '',
    
    -- 群聊内容字段
    -- 存储格式: message-sender-id:user-001,message-content:大家好,send-date:2025-01-15,message-type:text,reply-to:;...
    "group-chat-data" TEXT DEFAULT '',
    
    -- 存储格式: muted-user-id:user-003,mute-reason:违规,mute-expire-time:2025-01-20,mute-by:user-001;...
    "group-muted-members" TEXT DEFAULT '',
    
    -- 存储格式: notice-title:群公告,notice-content:欢迎加入,notice-publish-time:2025-01-10,notice-publisher-id:user-001;...
    "group-notice" TEXT DEFAULT '',
    
    -- 存储格式: file-name:文档.pdf,file-type:pdf,file-size:2048,upload-time:2025-01-15,file-path-url:minio://bucket/doc.pdf,uploader-id:user-001;...
    "group-chat-file-data-and-url" TEXT DEFAULT '',
    
    -- 时间戳
    "created-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ========================================
-- 用户刷新Token管理表 (user-refresh-tokens)
-- 支持多设备登录，每个设备独立的Refresh Token
-- ========================================
CREATE TABLE IF NOT EXISTS "user-refresh-tokens" (
    "token-id" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "refresh-token" TEXT NOT NULL UNIQUE,
    "device-id" TEXT NOT NULL,
    "device-info" TEXT,  -- JSON格式: {"device_type":"mobile","os":"Android 14",...}
    "ip-address" TEXT,
    "created-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "expires-at" TIMESTAMP NOT NULL,
    "last-used-at" TIMESTAMP,
    "is-revoked" BOOLEAN DEFAULT false,
    "revoked-at" TIMESTAMP,
    "revoked-reason" TEXT,
    
    -- 外键约束
    CONSTRAINT "fk-refresh-tokens-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- Token黑名单表 (token-blacklist)
-- 用于紧急撤销Access Token（15分钟有效期内需要立即失效）
-- ========================================
CREATE TABLE IF NOT EXISTS "token-blacklist" (
    "jti" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "token-type" TEXT NOT NULL,  -- "access" 或 "refresh"
    "expires-at" TIMESTAMP NOT NULL,
    "blacklisted-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "reason" TEXT,
    
    -- 外键约束
    CONSTRAINT "fk-blacklist-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- 用户Access Token缓存表 (user-access-cache)
-- 记录近15分钟签发的Access Token用于按设备精准拉黑
-- ========================================
CREATE TABLE IF NOT EXISTS "user-access-cache" (
    "jti" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "device-id" TEXT NOT NULL,
    "exp" TIMESTAMP NOT NULL,
    "issued-at" TIMESTAMP NOT NULL,
    "inserted-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "fk-access-cache-user"
        FOREIGN KEY ("user-id")
        REFERENCES "users"("user-id")
        ON DELETE CASCADE
);

-- ========================================
-- 创建索引 (提升查询性能)
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

-- Refresh Token表索引
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-user-id" ON "user-refresh-tokens"("user-id");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-token" ON "user-refresh-tokens"("refresh-token");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-expires" ON "user-refresh-tokens"("expires-at");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-device" ON "user-refresh-tokens"("device-id");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-revoked" ON "user-refresh-tokens"("is-revoked") WHERE "is-revoked" = false;

-- Token黑名单表索引
CREATE INDEX IF NOT EXISTS "idx-blacklist-user-id" ON "token-blacklist"("user-id");
CREATE INDEX IF NOT EXISTS "idx-blacklist-expires" ON "token-blacklist"("expires-at");
CREATE INDEX IF NOT EXISTS "idx-blacklist-type" ON "token-blacklist"("token-type");

-- Access Token缓存表索引
CREATE INDEX IF NOT EXISTS "idx-access-cache-user-device" ON "user-access-cache"("user-id", "device-id");
CREATE INDEX IF NOT EXISTS "idx-access-cache-exp" ON "user-access-cache"("exp");

-- ========================================
-- 创建触发器 (自动更新 updated-at)
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
-- 测试数据（可选）
-- ========================================
-- 如需添加测试数据，请在此处插入

-- ========================================
-- 创建视图 (便于查询)
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
-- 数据库初始化完成
-- ========================================

-- 显示表结构
\echo '========================================';
\echo '数据库初始化完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. users                  - 用户数据表';
\echo '  2. groups                 - 群聊数据表';
\echo '  3. user-refresh-tokens    - 刷新Token管理表（多设备）';
\echo '  4. token-blacklist        - Token黑名单表';
\echo '  5. user-access-cache      - Access Token缓存表';
\echo '';
\echo '创建的索引:';
\echo '  用户表:';
\echo '    - idx-users-email';
\echo '    - idx-users-nickname';
\echo '    - idx-users-admin';
\echo '    - idx-users-blacklist-check';
\echo '  群聊表:';
\echo '    - idx-groups-name';
\echo '  认证表:';
\echo '    - idx-refresh-tokens-user-id';
\echo '    - idx-refresh-tokens-token';
\echo '    - idx-refresh-tokens-expires';
\echo '    - idx-refresh-tokens-device';
\echo '    - idx-blacklist-user-id';
\echo '    - idx-blacklist-expires';
\echo '    - idx-access-cache-user-device';
\echo '    - idx-access-cache-exp';
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

-- 查询验证
SELECT '用户总数: ' || COUNT(*) FROM "users";
SELECT '群聊总数: ' || COUNT(*) FROM "groups";
SELECT '刷新Token总数: ' || COUNT(*) FROM "user-refresh-tokens";
SELECT '活跃设备数: ' || COUNT(*) FROM "user-refresh-tokens" WHERE "is-revoked" = false AND "expires-at" > NOW();

