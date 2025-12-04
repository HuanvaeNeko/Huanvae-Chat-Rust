-- ========================================
-- HuanVae Chat 消息优化表结构
-- 创建时间: 2025-12-04
-- 包含: 复合索引优化、消息缓存表、消息归档表
-- ========================================

-- ========================================
-- 1. 消息查询复合索引优化
-- ========================================

-- 好友消息：会话+时间复合索引（优化分页查询）
CREATE INDEX IF NOT EXISTS "idx-friend-messages-conv-time"
ON "friend-messages"("conversation-uuid", "send-time" DESC);

-- 群消息：群ID+时间复合索引（优化分页查询）
CREATE INDEX IF NOT EXISTS "idx-group-messages-group-time"
ON "group-messages"("group-id", "send-time" DESC);

-- 群消息：发送者+时间索引（优化 JOIN 查询）
CREATE INDEX IF NOT EXISTS "idx-group-messages-sender-time"
ON "group-messages"("sender-id", "send-time" DESC);

-- ========================================
-- 2. 群消息热点缓存表
-- 类似 user-access-cache 的设计思路
-- ========================================

CREATE TABLE IF NOT EXISTS "group-message-cache" (
    -- 缓存键
    "cache-id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL,
    
    -- 缓存的消息数据 (JSONB 存储最近N条消息)
    "messages" JSONB NOT NULL DEFAULT '[]'::jsonb,
    "message-count" INTEGER DEFAULT 0,
    
    -- 缓存元数据
    "last-message-uuid" UUID,
    "last-message-time" TIMESTAMPTZ,
    "oldest-message-time" TIMESTAMPTZ,
    
    -- TTL 管理
    "expires-at" TIMESTAMPTZ NOT NULL,
    "created-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT "uk-group-message-cache" UNIQUE ("group-id")
);

-- 缓存表索引
CREATE INDEX IF NOT EXISTS "idx-group-message-cache-expires" 
ON "group-message-cache"("expires-at");

CREATE INDEX IF NOT EXISTS "idx-group-message-cache-group-time"
ON "group-message-cache"("group-id", "last-message-time" DESC);

-- ========================================
-- 3. 消息归档表
-- ========================================

-- 好友消息归档表（结构与主表相同）
CREATE TABLE IF NOT EXISTS "friend-messages-archive" (
    "message-uuid" TEXT PRIMARY KEY,
    "conversation-uuid" TEXT NOT NULL,
    "sender-id" TEXT NOT NULL,
    "receiver-id" TEXT NOT NULL,
    "message-content" TEXT NOT NULL,
    "message-type" TEXT NOT NULL DEFAULT 'text',
    "file-uuid" TEXT,
    "file-url" TEXT,
    "file-size" BIGINT,
    "send-time" TIMESTAMPTZ NOT NULL,
    "is-deleted-by-sender" BOOLEAN DEFAULT false,
    "is-deleted-by-receiver" BOOLEAN DEFAULT false
);

-- 好友归档表索引
CREATE INDEX IF NOT EXISTS "idx-friend-messages-archive-conv-time"
ON "friend-messages-archive"("conversation-uuid", "send-time" DESC);

-- 群消息归档表（结构与主表相同）
CREATE TABLE IF NOT EXISTS "group-messages-archive" (
    "message-uuid" UUID PRIMARY KEY,
    "group-id" UUID NOT NULL,
    "sender-id" TEXT NOT NULL,
    "message-content" TEXT NOT NULL,
    "message-type" TEXT NOT NULL DEFAULT 'text',
    "file-uuid" TEXT,
    "file-url" TEXT,
    "file-size" BIGINT,
    "reply-to" UUID,
    "send-time" TIMESTAMPTZ NOT NULL,
    "is-recalled" BOOLEAN DEFAULT false,
    "recalled-at" TIMESTAMPTZ,
    "recalled-by" TEXT
);

-- 群归档表索引
CREATE INDEX IF NOT EXISTS "idx-group-messages-archive-group-time"
ON "group-messages-archive"("group-id", "send-time" DESC);

-- ========================================
-- 4. 消息归档函数
-- ========================================

CREATE OR REPLACE FUNCTION archive_old_messages(archive_days INTEGER DEFAULT 30)
RETURNS TABLE (
    friend_archived BIGINT,
    group_archived BIGINT
) AS $$
DECLARE
    cutoff_time TIMESTAMPTZ := NOW() - (archive_days || ' days')::INTERVAL;
    friend_count BIGINT := 0;
    group_count BIGINT := 0;
BEGIN
    -- 归档好友消息
    WITH moved AS (
        DELETE FROM "friend-messages"
        WHERE "send-time" < cutoff_time
        RETURNING *
    )
    INSERT INTO "friend-messages-archive" 
    SELECT * FROM moved
    ON CONFLICT ("message-uuid") DO NOTHING;
    GET DIAGNOSTICS friend_count = ROW_COUNT;
    
    -- 归档群消息
    WITH moved AS (
        DELETE FROM "group-messages"
        WHERE "send-time" < cutoff_time
        RETURNING *
    )
    INSERT INTO "group-messages-archive" 
    SELECT * FROM moved
    ON CONFLICT ("message-uuid") DO NOTHING;
    GET DIAGNOSTICS group_count = ROW_COUNT;
    
    RETURN QUERY SELECT friend_count, group_count;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 5. 缓存清理函数
-- ========================================

CREATE OR REPLACE FUNCTION cleanup_expired_message_cache()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM "group-message-cache" WHERE "expires-at" < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 添加注释
-- ========================================

COMMENT ON TABLE "group-message-cache" IS '群消息热点缓存表，存储每个群最近消息的JSONB快照';
COMMENT ON TABLE "friend-messages-archive" IS '好友消息归档表，存储30天前的历史消息';
COMMENT ON TABLE "group-messages-archive" IS '群消息归档表，存储30天前的历史消息';
COMMENT ON FUNCTION archive_old_messages IS '归档指定天数前的消息，返回归档数量';
COMMENT ON FUNCTION cleanup_expired_message_cache IS '清理过期的消息缓存';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '消息优化表结构创建完成！';
\echo '========================================';
\echo '';
\echo '创建的索引:';
\echo '  1. idx-friend-messages-conv-time    - 好友消息会话+时间索引';
\echo '  2. idx-group-messages-group-time    - 群消息群ID+时间索引';
\echo '  3. idx-group-messages-sender-time   - 群消息发送者+时间索引';
\echo '';
\echo '创建的表:';
\echo '  1. group-message-cache              - 群消息缓存表';
\echo '  2. friend-messages-archive          - 好友消息归档表';
\echo '  3. group-messages-archive           - 群消息归档表';
\echo '';
\echo '创建的函数:';
\echo '  1. archive_old_messages(days)       - 消息归档函数';
\echo '  2. cleanup_expired_message_cache()  - 缓存清理函数';
\echo '========================================';

