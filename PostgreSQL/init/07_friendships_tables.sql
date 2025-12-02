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
    "user_id" TEXT NOT NULL,
    "friend_id" TEXT NOT NULL,
    "status" TEXT NOT NULL DEFAULT 'active',  -- active, ended
    "remark" TEXT DEFAULT '',                  -- 好友备注
    "add_time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "end_time" TIMESTAMPTZ,                    -- 结束时间（删除好友时）
    "end_reason" TEXT,                         -- 结束原因
    
    -- 外键约束
    CONSTRAINT "fk_friendships_user"
        FOREIGN KEY ("user_id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    CONSTRAINT "fk_friendships_friend"
        FOREIGN KEY ("friend_id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    
    -- 唯一约束：同一对用户只能有一条记录
    CONSTRAINT "unique_friendship" UNIQUE ("user_id", "friend_id")
);

-- ========================================
-- 好友请求表 (friend_requests)
-- 存储好友申请记录
-- ========================================
CREATE TABLE IF NOT EXISTS "friend_requests" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "from_user_id" TEXT NOT NULL,              -- 申请人
    "to_user_id" TEXT NOT NULL,                -- 被申请人
    "message" TEXT DEFAULT '',                 -- 申请消息
    "status" TEXT NOT NULL DEFAULT 'pending',  -- pending, approved, rejected
    "reject_reason" TEXT,                      -- 拒绝原因
    "created-at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键约束
    CONSTRAINT "fk_friend_requests_from"
        FOREIGN KEY ("from_user_id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE,
    CONSTRAINT "fk_friend_requests_to"
        FOREIGN KEY ("to_user_id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- 创建索引
-- ========================================

-- 好友关系表索引
CREATE INDEX IF NOT EXISTS "idx_friendships_user_id" ON "friendships"("user_id");
CREATE INDEX IF NOT EXISTS "idx_friendships_friend_id" ON "friendships"("friend_id");
CREATE INDEX IF NOT EXISTS "idx_friendships_status" ON "friendships"("status") WHERE "status" = 'active';
CREATE INDEX IF NOT EXISTS "idx_friendships_user_status" ON "friendships"("user_id", "status");

-- 好友请求表索引
CREATE INDEX IF NOT EXISTS "idx_friend_requests_from" ON "friend_requests"("from_user_id");
CREATE INDEX IF NOT EXISTS "idx_friend_requests_to" ON "friend_requests"("to_user_id");
CREATE INDEX IF NOT EXISTS "idx_friend_requests_status" ON "friend_requests"("status");
CREATE INDEX IF NOT EXISTS "idx_friend_requests_to_status" ON "friend_requests"("to_user_id", "status") WHERE "status" = 'pending';
CREATE INDEX IF NOT EXISTS "idx_friend_requests_from_status" ON "friend_requests"("from_user_id", "status") WHERE "status" = 'pending';

-- ========================================
-- 创建触发器 (自动更新 updated_at)
-- ========================================
DROP TRIGGER IF EXISTS "trigger_update_friend_requests_timestamp" ON "friend_requests";
CREATE TRIGGER "trigger_update_friend_requests_timestamp"
BEFORE UPDATE ON "friend_requests"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- ========================================
-- 添加注释
-- ========================================
COMMENT ON TABLE "friendships" IS '好友关系表，存储双向好友关系';
COMMENT ON COLUMN "friendships"."status" IS '状态: active(有效), ended(已结束)';
COMMENT ON COLUMN "friendships"."remark" IS '好友备注名';

COMMENT ON TABLE "friend_requests" IS '好友请求表，存储好友申请记录';
COMMENT ON COLUMN "friend_requests"."status" IS '状态: pending(待处理), approved(已同意), rejected(已拒绝)';
COMMENT ON COLUMN "friend_requests"."message" IS '申请消息，支持特殊字符';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '好友关系表创建完成！';
\echo '========================================';
\echo '';
\echo '新增表:';
\echo '  1. friendships      - 好友关系表';
\echo '  2. friend_requests  - 好友请求表';
\echo '';
\echo '新增索引:';
\echo '  好友关系表:';
\echo '    - idx_friendships_user_id';
\echo '    - idx_friendships_friend_id';
\echo '    - idx_friendships_status';
\echo '    - idx_friendships_user_status';
\echo '';
\echo '  好友请求表:';
\echo '    - idx_friend_requests_from';
\echo '    - idx_friend_requests_to';
\echo '    - idx_friend_requests_status';
\echo '    - idx_friend_requests_to_status';
\echo '    - idx_friend_requests_from_status';
\echo '';
\echo '========================================';

