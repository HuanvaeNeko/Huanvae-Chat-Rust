-- ========================================
-- 添加好友消息相关表
-- 创建时间: 2025-11-27
-- ========================================

-- ========================================
-- 好友消息表 (friend-messages)
-- 存储用户之间的所有聊天消息
-- ========================================
CREATE TABLE IF NOT EXISTS "friend-messages" (
    -- 消息标识
    "message-uuid" TEXT PRIMARY KEY,
    "conversation-uuid" TEXT NOT NULL,  -- 会话唯一标识(双方用户ID排序后组合)
    
    -- 用户信息
    "sender-id" TEXT NOT NULL,
    "receiver-id" TEXT NOT NULL,
    
    -- 消息内容
    "message-content" TEXT NOT NULL,
    "message-type" TEXT NOT NULL,  -- 消息类型: text/image/video/file
    
    -- 文件信息(媒体消息使用)
    "file-url" TEXT,               -- MinIO文件URL
    "file-size" BIGINT,            -- 文件大小(字节)
    
    -- 时间信息(UTC时间)
    "send-time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- 删除标记(软删除)
    "is-deleted-by-sender" BOOLEAN DEFAULT false,
    "is-deleted-by-receiver" BOOLEAN DEFAULT false,
    
    -- 外键约束
    CONSTRAINT "fk-messages-sender"
        FOREIGN KEY ("sender-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    CONSTRAINT "fk-messages-receiver"
        FOREIGN KEY ("receiver-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- 未读消息表 (friend-unread-messages)
-- 存储每个用户与好友的会话信息和未读计数
-- ========================================
CREATE TABLE IF NOT EXISTS "friend-unread-messages" (
    -- 记录标识
    "unread-id" TEXT PRIMARY KEY,
    
    -- 用户和会话信息
    "user-id" TEXT NOT NULL,
    "conversation-uuid" TEXT NOT NULL,
    "friend-id" TEXT NOT NULL,
    
    -- 未读计数
    "unread-count" INTEGER DEFAULT 0,
    
    -- 最后一条消息信息(用于会话列表预览)
    "last-message-uuid" TEXT,
    "last-message-content" TEXT,
    "last-message-type" TEXT,
    "last-message-time" TIMESTAMPTZ,
    
    -- 更新时间
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键约束
    CONSTRAINT "fk-unread-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    
    -- 唯一约束(每个用户对每个会话只有一条记录)
    CONSTRAINT "unique-user-conversation"
        UNIQUE ("user-id", "conversation-uuid")
);

-- ========================================
-- 创建索引 (提升查询性能)
-- ========================================

-- 好友消息表索引
CREATE INDEX IF NOT EXISTS "idx-messages-conversation-time" 
    ON "friend-messages"("conversation-uuid", "send-time" DESC);

CREATE INDEX IF NOT EXISTS "idx-messages-sender" 
    ON "friend-messages"("sender-id");

CREATE INDEX IF NOT EXISTS "idx-messages-receiver" 
    ON "friend-messages"("receiver-id");

CREATE INDEX IF NOT EXISTS "idx-messages-send-time" 
    ON "friend-messages"("send-time" DESC);

CREATE INDEX IF NOT EXISTS "idx-messages-conversation" 
    ON "friend-messages"("conversation-uuid");

-- 未读消息表索引
CREATE INDEX IF NOT EXISTS "idx-unread-user-updated" 
    ON "friend-unread-messages"("user-id", "updated-at" DESC);

CREATE INDEX IF NOT EXISTS "idx-unread-conversation" 
    ON "friend-unread-messages"("conversation-uuid");

CREATE INDEX IF NOT EXISTS "idx-unread-count" 
    ON "friend-unread-messages"("unread-count") 
    WHERE "unread-count" > 0;

-- ========================================
-- 创建触发器 (自动更新 updated-at)
-- ========================================

-- 未读消息表更新触发器
DROP TRIGGER IF EXISTS "trigger-update-unread-timestamp" ON "friend-unread-messages";
CREATE TRIGGER "trigger-update-unread-timestamp"
BEFORE UPDATE ON "friend-unread-messages"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- ========================================
-- 显示更新信息
-- ========================================
\echo '========================================';
\echo '好友消息相关表创建完成！';
\echo '========================================';
\echo '';
\echo '新增表:';
\echo '  1. friend-messages        - 好友消息表';
\echo '  2. friend-unread-messages - 未读消息表';
\echo '';
\echo '新增索引:';
\echo '  好友消息表:';
\echo '    - idx-messages-conversation-time (主查询索引)';
\echo '    - idx-messages-sender';
\echo '    - idx-messages-receiver';
\echo '    - idx-messages-send-time';
\echo '    - idx-messages-conversation';
\echo '';
\echo '  未读消息表:';
\echo '    - idx-unread-user-updated (会话列表查询)';
\echo '    - idx-unread-conversation';
\echo '    - idx-unread-count (未读数查询)';
\echo '';
\echo '新增触发器:';
\echo '  - trigger-update-unread-timestamp';
\echo '';
\echo '========================================';


