-- ========================================
-- 好友关系表结构
-- 创建时间: 2025-12-02
-- 替代原 users 表中的 TEXT 格式存储
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

-- ========================================
-- 创建触发器 (自动更新 updated-at)
-- ========================================
DROP TRIGGER IF EXISTS "trigger-update-friend-requests-timestamp" ON "friend-requests";
CREATE TRIGGER "trigger-update-friend-requests-timestamp"
BEFORE UPDATE ON "friend-requests"
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
\echo '好友关系表创建完成！';
\echo '========================================';
\echo '';
\echo '新增表:';
\echo '  1. friendships      - 好友关系表';
\echo '  2. friend-requests  - 好友请求表';
\echo '';
\echo '新增索引:';
\echo '  好友关系表:';
\echo '    - idx-friendships-user-id';
\echo '    - idx-friendships-friend-id';
\echo '    - idx-friendships-status';
\echo '    - idx-friendships-user-status';
\echo '';
\echo '  好友请求表:';
\echo '    - idx-friend-requests-from';
\echo '    - idx-friend-requests-to';
\echo '    - idx-friend-requests-status';
\echo '    - idx-friend-requests-to-status';
\echo '    - idx-friend-requests-from-status';
\echo '';
\echo '========================================';
