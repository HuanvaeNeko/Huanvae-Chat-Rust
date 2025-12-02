-- 文件UUID映射表（去重核心）
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

CREATE INDEX IF NOT EXISTS "idx-file-hash-mapping" ON "file-uuid-mapping"("file-hash");
CREATE INDEX IF NOT EXISTS "idx-physical-key" ON "file-uuid-mapping"("physical-file-key");

-- 文件访问权限表
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

CREATE INDEX IF NOT EXISTS "idx-file-user-access" ON "file-access-permissions"("file-uuid", "user-id", "revoked-at");
CREATE INDEX IF NOT EXISTS "idx-user-files" ON "file-access-permissions"("user-id", "revoked-at");

-- 添加注释
COMMENT ON TABLE "file-uuid-mapping" IS 'UUID映射表，实现文件去重';
COMMENT ON COLUMN "file-uuid-mapping"."uuid" IS '随机UUID，用于访问URL';
COMMENT ON COLUMN "file-uuid-mapping"."physical-file-key" IS '实际物理文件路径';
COMMENT ON COLUMN "file-uuid-mapping"."file-hash" IS 'SHA-256哈希，用于去重查询';

COMMENT ON TABLE "file-access-permissions" IS '文件访问权限表';
COMMENT ON COLUMN "file-access-permissions"."file-uuid" IS '关联映射表的UUID';
COMMENT ON COLUMN "file-access-permissions"."access-type" IS '权限类型：owner/read';
COMMENT ON COLUMN "file-access-permissions"."granted-by" IS '授权来源：upload/share';
COMMENT ON COLUMN "file-access-permissions"."revoked-at" IS '软删除时间';
