-- ========================================
-- HuanVae Chat 文件系统表结构
-- 创建时间: 2025-12-03
-- 包含: file-records, file-uuid-mapping, file-access-permissions, user-storage-quotas
-- ========================================

-- ========================================
-- 文件记录表 (file-records)
-- 存储所有上传文件的元数据
-- ========================================
CREATE TABLE IF NOT EXISTS "file-records" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "file-key" VARCHAR(500) NOT NULL UNIQUE,
    "file-url" TEXT,
    "file-type" VARCHAR(50) NOT NULL,
    "storage-location" VARCHAR(50) NOT NULL,
    "preview-support" VARCHAR(20) NOT NULL DEFAULT 'download_only',
    
    -- 所有者信息
    "owner-id" VARCHAR(255) NOT NULL,
    "related-id" VARCHAR(255),
    
    -- 文件信息
    "file-size" BIGINT NOT NULL,
    "actual-size" BIGINT,
    "content-type" VARCHAR(100) NOT NULL,
    
    -- 哈希值（去重和完整性校验）
    "file-hash" VARCHAR(64) NOT NULL,
    
    -- 物理文件引用（用于去重）
    "physical-file-key" VARCHAR(500),
    
    -- 上传凭证
    "upload-token" VARCHAR(128),
    "multipart-upload-id" VARCHAR(200),
    
    -- UUID映射
    "file-uuid" VARCHAR(36),
    
    -- 状态管理
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "created-at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expires-at" TIMESTAMPTZ NOT NULL,
    "completed-at" TIMESTAMPTZ,
    "deleted-at" TIMESTAMPTZ
);

-- ========================================
-- 文件UUID映射表 (file-uuid-mapping)
-- UUID映射表，实现文件去重
-- ========================================
CREATE TABLE IF NOT EXISTS "file-uuid-mapping" (
    "uuid" VARCHAR(36) PRIMARY KEY,
    "physical-file-key" VARCHAR(500) NOT NULL,
    "file-hash" VARCHAR(64) NOT NULL,
    "file-size" BIGINT NOT NULL,
    "content-type" VARCHAR(100) NOT NULL,
    "preview-support" VARCHAR(20) NOT NULL,
    "created-at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "first-uploader-id" VARCHAR(255) NOT NULL
);

-- ========================================
-- 文件访问权限表 (file-access-permissions)
-- 控制用户对文件的访问权限
-- ========================================
CREATE TABLE IF NOT EXISTS "file-access-permissions" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "file-uuid" VARCHAR(36) NOT NULL,
    "user-id" VARCHAR(255) NOT NULL,
    "access-type" VARCHAR(20) NOT NULL DEFAULT 'owner',
    "granted-by" VARCHAR(50) NOT NULL,
    "related-context" VARCHAR(500),
    "granted-at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "revoked-at" TIMESTAMPTZ
);

-- ========================================
-- 用户存储配额表 (user-storage-quotas)
-- 管理用户的存储配额和使用情况
-- ========================================
CREATE TABLE IF NOT EXISTS "user-storage-quotas" (
    "user-id" VARCHAR(255) PRIMARY KEY,
    "total-quota" BIGINT NOT NULL DEFAULT 10737418240, -- 10GB默认
    "used-space" BIGINT NOT NULL DEFAULT 0,
    "file-count" INTEGER NOT NULL DEFAULT 0,
    "updated-at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ========================================
-- 创建索引
-- ========================================

-- file-records 表索引
CREATE INDEX IF NOT EXISTS "idx-file-hash" ON "file-records"("file-hash");
CREATE INDEX IF NOT EXISTS "idx-owner-id" ON "file-records"("owner-id");
CREATE INDEX IF NOT EXISTS "idx-status-expires" ON "file-records"("status", "expires-at");
CREATE INDEX IF NOT EXISTS "idx-upload-token" ON "file-records"("upload-token") WHERE "upload-token" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-related-id" ON "file-records"("related-id") WHERE "related-id" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-file-uuid" ON "file-records"("file-uuid");

-- file-uuid-mapping 表索引
CREATE INDEX IF NOT EXISTS "idx-file-hash-mapping" ON "file-uuid-mapping"("file-hash");
CREATE INDEX IF NOT EXISTS "idx-physical-key" ON "file-uuid-mapping"("physical-file-key");

-- file-access-permissions 表索引
CREATE INDEX IF NOT EXISTS "idx-file-user-access" ON "file-access-permissions"("file-uuid", "user-id", "revoked-at");
CREATE INDEX IF NOT EXISTS "idx-user-files" ON "file-access-permissions"("user-id", "revoked-at");

-- ========================================
-- 创建触发器
-- ========================================

-- 文件完成时更新配额的触发器函数
CREATE OR REPLACE FUNCTION update_user_quota()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW."status" = 'completed' AND OLD."status" != 'completed' THEN
        INSERT INTO "user-storage-quotas" ("user-id", "used-space", "file-count")
        VALUES (NEW."owner-id", NEW."file-size", 1)
        ON CONFLICT ("user-id") DO UPDATE
        SET "used-space" = "user-storage-quotas"."used-space" + NEW."file-size",
            "file-count" = "user-storage-quotas"."file-count" + 1,
            "updated-at" = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trigger-update-quota" ON "file-records";
CREATE TRIGGER "trigger-update-quota"
AFTER UPDATE ON "file-records"
FOR EACH ROW
EXECUTE FUNCTION update_user_quota();

-- ========================================
-- 添加注释
-- ========================================
COMMENT ON TABLE "file-records" IS '文件记录表，存储所有上传文件的元数据';
COMMENT ON COLUMN "file-records"."file-hash" IS 'SHA-256哈希值，用于去重和完整性校验';
COMMENT ON COLUMN "file-records"."upload-token" IS '一次性上传Token，上传完成后清空';
COMMENT ON COLUMN "file-records"."preview-support" IS '预览支持：inline_preview 或 download_only';
COMMENT ON COLUMN "file-records"."status" IS '状态：pending（待上传）、completed（已完成）、failed（失败）';

COMMENT ON TABLE "file-uuid-mapping" IS 'UUID映射表，实现文件去重';
COMMENT ON COLUMN "file-uuid-mapping"."uuid" IS '随机UUID，用于访问URL';
COMMENT ON COLUMN "file-uuid-mapping"."physical-file-key" IS '实际物理文件路径';
COMMENT ON COLUMN "file-uuid-mapping"."file-hash" IS 'SHA-256哈希，用于去重查询';

COMMENT ON TABLE "file-access-permissions" IS '文件访问权限表';
COMMENT ON COLUMN "file-access-permissions"."file-uuid" IS '关联映射表的UUID';
COMMENT ON COLUMN "file-access-permissions"."access-type" IS '权限类型：owner/read';
COMMENT ON COLUMN "file-access-permissions"."granted-by" IS '授权来源：upload/share';
COMMENT ON COLUMN "file-access-permissions"."revoked-at" IS '软删除时间';

COMMENT ON TABLE "user-storage-quotas" IS '用户存储配额表';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '文件系统表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. file-records            - 文件记录表';
\echo '  2. file-uuid-mapping       - UUID映射表';
\echo '  3. file-access-permissions - 文件访问权限表';
\echo '  4. user-storage-quotas     - 用户存储配额表';
\echo '';
\echo '创建的索引:';
\echo '  file-records:            6个索引';
\echo '  file-uuid-mapping:       2个索引';
\echo '  file-access-permissions: 2个索引';
\echo '';
\echo '创建的触发器:';
\echo '  - trigger-update-quota';
\echo '';
\echo '========================================';

