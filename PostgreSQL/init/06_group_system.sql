-- ========================================
-- HuanVae Chat 群聊系统表结构
-- 创建时间: 2025-12-03
-- 包含: 群聊核心表、成员表、入群申请表、邀请码表、公告表、消息表等
-- ========================================

-- ========================================
-- 1. 改造 groups 表（添加新字段）
-- ========================================

-- 先删除旧的 groups 表（如果存在旧版本）
DROP TABLE IF EXISTS "group-message-deletions" CASCADE;
DROP TABLE IF EXISTS "group-messages" CASCADE;
DROP TABLE IF EXISTS "group-unread-messages" CASCADE;
DROP TABLE IF EXISTS "group-notices" CASCADE;
DROP TABLE IF EXISTS "group-invite-codes" CASCADE;
DROP TABLE IF EXISTS "group-join-requests" CASCADE;
DROP TABLE IF EXISTS "group-members" CASCADE;

-- 删除旧的 groups 表并重建
DROP TABLE IF EXISTS "groups" CASCADE;

CREATE TABLE IF NOT EXISTS "groups" (
    -- 基础字段
    "group-id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-name" TEXT NOT NULL,
    "group-avatar-url" TEXT DEFAULT '',
    "group-description" TEXT DEFAULT '',
    
    -- 创建信息
    "creator-id" TEXT NOT NULL,
    "created-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- 入群设置
    -- 可选值: open, approval_required, invite_only, admin_invite_only, closed
    "join-mode" TEXT NOT NULL DEFAULT 'approval_required',
    
    -- 群状态
    "status" TEXT NOT NULL DEFAULT 'active',
    "disbanded-at" TIMESTAMPTZ,
    "disbanded-by" TEXT,
    
    -- 统计字段
    "member-count" INTEGER DEFAULT 1,
    
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 群表索引
CREATE INDEX IF NOT EXISTS "idx-groups-creator" ON "groups"("creator-id");
CREATE INDEX IF NOT EXISTS "idx-groups-status" ON "groups"("status") WHERE "status" = 'active';
CREATE INDEX IF NOT EXISTS "idx-groups-created-at" ON "groups"("created-at" DESC);
CREATE INDEX IF NOT EXISTS "idx-groups-name" ON "groups"("group-name");

-- 群表注释
COMMENT ON TABLE "groups" IS '群聊主表';
COMMENT ON COLUMN "groups"."group-id" IS '群聊唯一标识（UUID）';
COMMENT ON COLUMN "groups"."group-name" IS '群名称';
COMMENT ON COLUMN "groups"."group-avatar-url" IS '群头像URL';
COMMENT ON COLUMN "groups"."group-description" IS '群简介';
COMMENT ON COLUMN "groups"."creator-id" IS '创建人ID';
COMMENT ON COLUMN "groups"."join-mode" IS '入群模式: open/approval_required/invite_only/admin_invite_only/closed';
COMMENT ON COLUMN "groups"."status" IS '群状态: active/disbanded';
COMMENT ON COLUMN "groups"."member-count" IS '成员数量';

-- ========================================
-- 2. 群成员表 (group-members)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-members" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    "user-id" TEXT NOT NULL,
    
    -- 角色: owner/admin/member
    "role" TEXT NOT NULL DEFAULT 'member',
    
    -- 群内昵称
    "group-nickname" TEXT,
    
    -- 加入信息
    "joined-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    -- 入群方式: create/owner_invite/admin_invite/member_invite/direct_invite_code/normal_invite_code/search_direct/search_approved
    "join-method" TEXT NOT NULL,
    "invited-by" TEXT,
    "approved-by" TEXT,
    "invite-code-id" UUID,
    
    -- 成员状态: active/removed/left
    "status" TEXT NOT NULL DEFAULT 'active',
    "left-at" TIMESTAMPTZ,
    "left-reason" TEXT,
    "removed-by" TEXT,
    "removed-reason" TEXT,
    
    -- 禁言信息
    "muted-until" TIMESTAMPTZ,
    "muted-by" TEXT,
    "muted-reason" TEXT,
    
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- 唯一约束
    CONSTRAINT "uq-group-member" UNIQUE ("group-id", "user-id")
);

-- 群成员表索引
CREATE INDEX IF NOT EXISTS "idx-group-members-group" ON "group-members"("group-id", "status");
CREATE INDEX IF NOT EXISTS "idx-group-members-user" ON "group-members"("user-id", "status");
CREATE INDEX IF NOT EXISTS "idx-group-members-role" ON "group-members"("group-id", "role") WHERE "status" = 'active';
CREATE INDEX IF NOT EXISTS "idx-group-members-muted" ON "group-members"("group-id", "muted-until") WHERE "muted-until" IS NOT NULL;

-- 群成员表注释
COMMENT ON TABLE "group-members" IS '群成员表';
COMMENT ON COLUMN "group-members"."role" IS '角色: owner/admin/member';
COMMENT ON COLUMN "group-members"."join-method" IS '入群方式';
COMMENT ON COLUMN "group-members"."status" IS '成员状态: active/removed/left';
COMMENT ON COLUMN "group-members"."muted-until" IS '禁言截止时间，NULL表示未禁言';

-- ========================================
-- 3. 入群申请/邀请表 (group-join-requests)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-join-requests" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    "user-id" TEXT NOT NULL,
    
    -- 请求类型: owner_invite/admin_invite/member_invite/direct_invite_code/normal_invite_code/search_apply
    "request-type" TEXT NOT NULL,
    
    -- 邀请/申请信息
    "inviter-id" TEXT,
    "invite-code-id" UUID,
    "message" TEXT,
    
    -- 审核流程状态
    "user-accepted" BOOLEAN DEFAULT false,
    "user-accepted-at" TIMESTAMPTZ,
    
    -- 整体状态: pending/approved/rejected/cancelled/expired
    "status" TEXT NOT NULL DEFAULT 'pending',
    
    -- 处理信息
    "processed-by" TEXT,
    "processed-at" TIMESTAMPTZ,
    "reject-reason" TEXT,
    
    -- 时间
    "created-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    "expires-at" TIMESTAMPTZ,
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 入群申请表索引
CREATE INDEX IF NOT EXISTS "idx-join-requests-group-status" ON "group-join-requests"("group-id", "status") WHERE "status" = 'pending';
CREATE INDEX IF NOT EXISTS "idx-join-requests-user" ON "group-join-requests"("user-id", "status");
CREATE INDEX IF NOT EXISTS "idx-join-requests-inviter" ON "group-join-requests"("inviter-id") WHERE "inviter-id" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-join-requests-expires" ON "group-join-requests"("expires-at") WHERE "status" = 'pending' AND "expires-at" IS NOT NULL;

-- 入群申请表注释
COMMENT ON TABLE "group-join-requests" IS '入群申请/邀请表';
COMMENT ON COLUMN "group-join-requests"."request-type" IS '请求类型';
COMMENT ON COLUMN "group-join-requests"."user-accepted" IS '被邀请人是否已同意';
COMMENT ON COLUMN "group-join-requests"."status" IS '状态: pending/approved/rejected/cancelled/expired';

-- ========================================
-- 4. 群邀请码表 (group-invite-codes)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-invite-codes" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    
    -- 邀请码信息
    "code" TEXT NOT NULL UNIQUE,
    -- 类型: direct（直通）/ normal（普通，需审核）
    "code-type" TEXT NOT NULL,
    
    -- 生成者信息
    "creator-id" TEXT NOT NULL,
    "creator-role" TEXT NOT NULL,
    
    -- 限制条件
    "max-uses" INTEGER,
    "used-count" INTEGER DEFAULT 0,
    "expires-at" TIMESTAMPTZ,
    
    -- 状态: active/expired/revoked/exhausted
    "status" TEXT NOT NULL DEFAULT 'active',
    "revoked-at" TIMESTAMPTZ,
    "revoked-by" TEXT,
    
    "created-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 邀请码表索引
CREATE INDEX IF NOT EXISTS "idx-invite-codes-group" ON "group-invite-codes"("group-id", "status");
CREATE INDEX IF NOT EXISTS "idx-invite-codes-code" ON "group-invite-codes"("code") WHERE "status" = 'active';
CREATE INDEX IF NOT EXISTS "idx-invite-codes-creator" ON "group-invite-codes"("creator-id");
CREATE INDEX IF NOT EXISTS "idx-invite-codes-expires" ON "group-invite-codes"("expires-at") WHERE "status" = 'active' AND "expires-at" IS NOT NULL;

-- 邀请码表注释
COMMENT ON TABLE "group-invite-codes" IS '群邀请码表';
COMMENT ON COLUMN "group-invite-codes"."code-type" IS '邀请码类型: direct/normal';
COMMENT ON COLUMN "group-invite-codes"."max-uses" IS '最大使用次数，NULL表示无限';
COMMENT ON COLUMN "group-invite-codes"."status" IS '状态: active/expired/revoked/exhausted';

-- ========================================
-- 5. 群公告表 (group-notices)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-notices" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    
    -- 公告内容
    "title" TEXT,
    "content" TEXT NOT NULL,
    
    -- 发布信息
    "publisher-id" TEXT NOT NULL,
    "published-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- 状态
    "is-pinned" BOOLEAN DEFAULT false,
    "is-active" BOOLEAN DEFAULT true,
    "deleted-at" TIMESTAMPTZ,
    "deleted-by" TEXT,
    
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 群公告表索引
CREATE INDEX IF NOT EXISTS "idx-notices-group" ON "group-notices"("group-id", "is-active", "published-at" DESC);
CREATE INDEX IF NOT EXISTS "idx-notices-pinned" ON "group-notices"("group-id", "is-pinned") WHERE "is-active" = true;

-- 群公告表注释
COMMENT ON TABLE "group-notices" IS '群公告表';
COMMENT ON COLUMN "group-notices"."is-pinned" IS '是否置顶';
COMMENT ON COLUMN "group-notices"."is-active" IS '是否有效';

-- ========================================
-- 6. 群消息表 (group-messages)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-messages" (
    "message-uuid" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    "sender-id" TEXT NOT NULL,
    
    -- 消息内容
    "message-content" TEXT NOT NULL,
    -- 消息类型: text/image/video/file/system
    "message-type" TEXT NOT NULL DEFAULT 'text',
    
    -- 文件信息
    "file-uuid" VARCHAR(36),
    "file-url" TEXT,
    "file-size" BIGINT,
    
    -- 回复功能
    "reply-to" UUID,
    
    -- 时间
    "send-time" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    -- 撤回标记
    "is-recalled" BOOLEAN DEFAULT false,
    "recalled-at" TIMESTAMPTZ,
    "recalled-by" TEXT
);

-- 群消息表索引
CREATE INDEX IF NOT EXISTS "idx-group-messages-group-time" ON "group-messages"("group-id", "send-time" DESC);
CREATE INDEX IF NOT EXISTS "idx-group-messages-sender" ON "group-messages"("sender-id");
CREATE INDEX IF NOT EXISTS "idx-group-messages-reply" ON "group-messages"("reply-to") WHERE "reply-to" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-group-messages-file" ON "group-messages"("file-uuid") WHERE "file-uuid" IS NOT NULL;

-- 群消息表注释
COMMENT ON TABLE "group-messages" IS '群消息表';
COMMENT ON COLUMN "group-messages"."message-type" IS '消息类型: text/image/video/file/system';
COMMENT ON COLUMN "group-messages"."reply-to" IS '回复的消息UUID';
COMMENT ON COLUMN "group-messages"."is-recalled" IS '是否已撤回';

-- ========================================
-- 7. 群消息删除记录表 (group-message-deletions)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-message-deletions" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "message-uuid" UUID NOT NULL REFERENCES "group-messages"("message-uuid") ON DELETE CASCADE,
    "user-id" TEXT NOT NULL,
    "deleted-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT "uq-message-deletion" UNIQUE ("message-uuid", "user-id")
);

-- 群消息删除记录表索引
CREATE INDEX IF NOT EXISTS "idx-message-deletions-user" ON "group-message-deletions"("user-id", "message-uuid");

-- 群消息删除记录表注释
COMMENT ON TABLE "group-message-deletions" IS '群消息删除记录表（个人删除，不影响他人）';

-- ========================================
-- 8. 群未读消息表 (group-unread-messages)
-- ========================================

CREATE TABLE IF NOT EXISTS "group-unread-messages" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "user-id" TEXT NOT NULL,
    "group-id" UUID NOT NULL REFERENCES "groups"("group-id") ON DELETE CASCADE,
    
    -- 未读信息
    "unread-count" INTEGER DEFAULT 0,
    
    -- 最后一条消息信息
    "last-message-uuid" UUID,
    "last-message-content" TEXT,
    "last-message-type" TEXT,
    "last-message-time" TIMESTAMPTZ,
    "last-sender-id" TEXT,
    
    "updated-at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT "uq-group-unread" UNIQUE ("user-id", "group-id")
);

-- 群未读消息表索引
CREATE INDEX IF NOT EXISTS "idx-group-unread-user" ON "group-unread-messages"("user-id", "last-message-time" DESC);
CREATE INDEX IF NOT EXISTS "idx-group-unread-count" ON "group-unread-messages"("user-id", "unread-count") WHERE "unread-count" > 0;

-- 群未读消息表注释
COMMENT ON TABLE "group-unread-messages" IS '群未读消息表';

-- ========================================
-- 9. 触发器：自动更新 updated-at
-- ========================================

-- groups 表触发器
DROP TRIGGER IF EXISTS "trigger-update-groups-timestamp" ON "groups";
CREATE TRIGGER "trigger-update-groups-timestamp"
BEFORE UPDATE ON "groups"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- group-members 表触发器
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW."updated-at" = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trigger-update-group-members-timestamp" ON "group-members";
CREATE TRIGGER "trigger-update-group-members-timestamp"
BEFORE UPDATE ON "group-members"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- group-join-requests 表触发器
DROP TRIGGER IF EXISTS "trigger-update-group-join-requests-timestamp" ON "group-join-requests";
CREATE TRIGGER "trigger-update-group-join-requests-timestamp"
BEFORE UPDATE ON "group-join-requests"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- group-invite-codes 表触发器
DROP TRIGGER IF EXISTS "trigger-update-group-invite-codes-timestamp" ON "group-invite-codes";
CREATE TRIGGER "trigger-update-group-invite-codes-timestamp"
BEFORE UPDATE ON "group-invite-codes"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- group-notices 表触发器
DROP TRIGGER IF EXISTS "trigger-update-group-notices-timestamp" ON "group-notices";
CREATE TRIGGER "trigger-update-group-notices-timestamp"
BEFORE UPDATE ON "group-notices"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- group-unread-messages 表触发器
DROP TRIGGER IF EXISTS "trigger-update-group-unread-messages-timestamp" ON "group-unread-messages";
CREATE TRIGGER "trigger-update-group-unread-messages-timestamp"
BEFORE UPDATE ON "group-unread-messages"
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- ========================================
-- 10. 视图：用户已加入的群聊
-- ========================================

CREATE OR REPLACE VIEW "view-user-groups" AS
SELECT 
    gm."user-id",
    g."group-id",
    g."group-name",
    g."group-avatar-url",
    gm."role",
    gm."group-nickname",
    gm."joined-at",
    gm."muted-until",
    gu."unread-count",
    gu."last-message-content",
    gu."last-message-time"
FROM "group-members" gm
JOIN "groups" g ON g."group-id" = gm."group-id"
LEFT JOIN "group-unread-messages" gu ON gu."group-id" = gm."group-id" AND gu."user-id" = gm."user-id"
WHERE gm."status" = 'active' AND g."status" = 'active';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '群聊系统表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. groups           - 群聊主表（已重建）';
\echo '  2. group-members    - 群成员表';
\echo '  3. group-join-requests - 入群申请/邀请表';
\echo '  4. group-invite-codes  - 邀请码表';
\echo '  5. group-notices    - 群公告表';
\echo '  6. group-messages   - 群消息表';
\echo '  7. group-message-deletions - 消息删除记录表';
\echo '  8. group-unread-messages   - 未读消息表';
\echo '';
\echo '创建的视图:';
\echo '  - view-user-groups  - 用户已加入的群聊视图';
\echo '';
\echo '========================================';

