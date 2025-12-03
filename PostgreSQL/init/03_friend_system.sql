-- ========================================
-- HuanVae Chat 好友系统表结构
-- 创建时间: 2025-12-03
-- 包含: friendships, friend-requests, friend-messages, friend-unread-messages
-- ========================================

-- ========================================
-- 好友关系表 (friendships)
-- 存储双向好友关系
-- ========================================
CREATE TABLE IF NOT EXISTS "friendships" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "user-id" TEXT NOT NULL,
    "friend-id" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'active',  -- active, ended
    "remark" TEXT DEFAULT '',                  -- 好友备注
    "add-time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "end-time" TIMESTAMPTZ,                    -- 结束时间（删除好友时）
    "end-reason" TEXT,                         -- 结束原因
    
    -- 外键约束
    CONSTRAINT "fk-friendships-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    CONSTRAINT "fk-friendships-friend"
        FOREIGN KEY ("friend-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    
    -- 唯一约束：同一对用户只能有一条记录
    CONSTRAINT "unique-friendship" UNIQUE ("user-id", "friend-id")
);

-- ========================================
-- 好友请求表 (friend-requests)
-- 存储好友申请记录
-- ========================================
CREATE TABLE IF NOT EXISTS "friend-requests" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "from-user-id" TEXT NOT NULL,              -- 申请人
    "to-user-id" TEXT NOT NULL,                -- 被申请人
    "message" TEXT DEFAULT '',                 -- 申请消息
    "status" TEXT NOT NULL DEFAULT 'pending',  -- pending, approved, rejected
    "reject-reason" TEXT,                      -- 拒绝原因
    "created-at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键约束
    CONSTRAINT "fk-friend-requests-from"
        FOREIGN KEY ("from-user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    CONSTRAINT "fk-friend-requests-to"
        FOREIGN KEY ("to-user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

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
    "file-url" TEXT,               -- 文件访问URL
    "file-uuid" VARCHAR(36),       -- 文件UUID，关联 file-uuid-mapping 表
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
-- 创建索引
-- ========================================

-- 好友关系表索引
CREATE INDEX IF NOT EXISTS "idx-friendships-user-id" ON "friendships"("user-id");
CREATE INDEX IF NOT EXISTS "idx-friendships-friend-id" ON "friendships"("friend-id");
CREATE INDEX IF NOT EXISTS "idx-friendships-status" ON "friendships"("status") WHERE "status" = 'active';
CREATE INDEX IF NOT EXISTS "idx-friendships-user-status" ON "friendships"("user-id", "status");

-- 好友请求表索引
CREATE INDEX IF NOT EXISTS "idx-friend-requests-from" ON "friend-requests"("from-user-id");
CREATE INDEX IF NOT EXISTS "idx-friend-requests-to" ON "friend-requests"("to-user-id");
CREATE INDEX IF NOT EXISTS "idx-friend-requests-status" ON "friend-requests"("status");
CREATE INDEX IF NOT EXISTS "idx-friend-requests-to-status" ON "friend-requests"("to-user-id", "status") WHERE "status" = 'pending';
CREATE INDEX IF NOT EXISTS "idx-friend-requests-from-status" ON "friend-requests"("from-user-id", "status") WHERE "status" = 'pending';

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
CREATE INDEX IF NOT EXISTS "idx-friend-messages-file-uuid" 
    ON "friend-messages"("file-uuid") 
    WHERE "file-uuid" IS NOT NULL;

-- 未读消息表索引
CREATE INDEX IF NOT EXISTS "idx-unread-user-updated" 
    ON "friend-unread-messages"("user-id", "updated-at" DESC);
CREATE INDEX IF NOT EXISTS "idx-unread-conversation" 
    ON "friend-unread-messages"("conversation-uuid");
CREATE INDEX IF NOT EXISTS "idx-unread-count" 
    ON "friend-unread-messages"("unread-count") 
    WHERE "unread-count" > 0;

-- ========================================
-- 创建触发器
-- ========================================

-- 好友请求表更新触发器
DROP TRIGGER IF EXISTS "trigger-update-friend-requests-timestamp" ON "friend-requests";
CREATE TRIGGER "trigger-update-friend-requests-timestamp"
BEFORE UPDATE ON "friend-requests"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- 未读消息表更新触发器
DROP TRIGGER IF EXISTS "trigger-update-unread-timestamp" ON "friend-unread-messages";
CREATE TRIGGER "trigger-update-unread-timestamp"
BEFORE UPDATE ON "friend-unread-messages"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- ========================================
-- 添加注释
-- ========================================
COMMENT ON TABLE "friendships" IS '好友关系表，存储双向好友关系';
COMMENT ON COLUMN "friendships"."status" IS '状态: active(有效), ended(已结束)';
COMMENT ON COLUMN "friendships"."remark" IS '好友备注名';

COMMENT ON TABLE "friend-requests" IS '好友请求表，存储好友申请记录';
COMMENT ON COLUMN "friend-requests"."status" IS '状态: pending(待处理), approved(已同意), rejected(已拒绝)';
COMMENT ON COLUMN "friend-requests"."message" IS '申请消息，支持特殊字符';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '好友系统表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. friendships             - 好友关系表';
\echo '  2. friend-requests         - 好友请求表';
\echo '  3. friend-messages         - 好友消息表';
\echo '  4. friend-unread-messages  - 未读消息表';
\echo '';
\echo '创建的索引:';
\echo '  friendships:             4个索引';
\echo '  friend-requests:         5个索引';
\echo '  friend-messages:         6个索引';
\echo '  friend-unread-messages:  3个索引';
\echo '';
\echo '创建的触发器:';
\echo '  - trigger-update-friend-requests-timestamp';
\echo '  - trigger-update-unread-timestamp';
\echo '';
\echo '========================================';

