-- 文件记录表
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

-- 索引
CREATE INDEX IF NOT EXISTS "idx-file-hash" ON "file-records"("file-hash");
CREATE INDEX IF NOT EXISTS "idx-owner-id" ON "file-records"("owner-id");
CREATE INDEX IF NOT EXISTS "idx-status-expires" ON "file-records"("status", "expires-at");
CREATE INDEX IF NOT EXISTS "idx-upload-token" ON "file-records"("upload-token") WHERE "upload-token" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-related-id" ON "file-records"("related-id") WHERE "related-id" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx-file-uuid" ON "file-records"("file-uuid");

-- 文件引用表（秒传时使用）
CREATE TABLE IF NOT EXISTS "file-references" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "original-file-key" VARCHAR(500) NOT NULL,
    "owner-id" VARCHAR(255) NOT NULL,
    "file-type" VARCHAR(50) NOT NULL,
    "file-hash" VARCHAR(64) NOT NULL,
    "created-at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "deleted-at" TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS "idx-ref-owner-file" ON "file-references"("owner-id", "original-file-key");
CREATE INDEX IF NOT EXISTS "idx-ref-hash" ON "file-references"("file-hash");

-- 用户存储配额表
CREATE TABLE IF NOT EXISTS "user-storage-quotas" (
    "user-id" VARCHAR(255) PRIMARY KEY,
    "total-quota" BIGINT NOT NULL DEFAULT 10737418240, -- 10GB默认
    "used-space" BIGINT NOT NULL DEFAULT 0,
    "file-count" INTEGER NOT NULL DEFAULT 0,
    "updated-at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 触发器：文件完成时更新配额
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

-- 添加注释
COMMENT ON TABLE "file-records" IS '文件记录表，存储所有上传文件的元数据';
COMMENT ON COLUMN "file-records"."file-hash" IS 'SHA-256哈希值，用于去重和完整性校验';
COMMENT ON COLUMN "file-records"."upload-token" IS '一次性上传Token，上传完成后清空';
COMMENT ON COLUMN "file-records"."preview-support" IS '预览支持：inline_preview 或 download_only';
COMMENT ON COLUMN "file-records"."status" IS '状态：pending（待上传）、completed（已完成）、failed（失败）';

COMMENT ON TABLE "file-references" IS '文件引用表，用于秒传功能';
COMMENT ON TABLE "user-storage-quotas" IS '用户存储配额表';
