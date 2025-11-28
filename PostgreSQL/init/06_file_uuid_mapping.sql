-- 文件UUID映射表（去重核心）
CREATE TABLE IF NOT EXISTS file_uuid_mapping (
    uuid VARCHAR(36) PRIMARY KEY,
    physical_file_key VARCHAR(500) NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    file_size BIGINT NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    preview_support VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    first_uploader_id VARCHAR(255) NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_file_hash_mapping ON file_uuid_mapping(file_hash);
CREATE INDEX IF NOT EXISTS idx_physical_key ON file_uuid_mapping(physical_file_key);

-- 文件访问权限表
CREATE TABLE IF NOT EXISTS file_access_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_uuid VARCHAR(36) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    access_type VARCHAR(20) NOT NULL DEFAULT 'owner',
    granted_by VARCHAR(50) NOT NULL,
    related_context VARCHAR(500),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_file_user_access ON file_access_permissions(file_uuid, user_id, revoked_at);
CREATE INDEX IF NOT EXISTS idx_user_files ON file_access_permissions(user_id, revoked_at);

-- 修改file_records表
ALTER TABLE file_records ADD COLUMN IF NOT EXISTS file_uuid VARCHAR(36);
CREATE INDEX IF NOT EXISTS idx_file_uuid ON file_records(file_uuid);

-- 添加注释
COMMENT ON TABLE file_uuid_mapping IS 'UUID映射表，实现文件去重';
COMMENT ON COLUMN file_uuid_mapping.uuid IS '随机UUID，用于访问URL';
COMMENT ON COLUMN file_uuid_mapping.physical_file_key IS '实际物理文件路径';
COMMENT ON COLUMN file_uuid_mapping.file_hash IS 'SHA-256哈希，用于去重查询';

COMMENT ON TABLE file_access_permissions IS '文件访问权限表';
COMMENT ON COLUMN file_access_permissions.file_uuid IS '关联映射表的UUID';
COMMENT ON COLUMN file_access_permissions.access_type IS '权限类型：owner/read';
COMMENT ON COLUMN file_access_permissions.granted_by IS '授权来源：upload/share';
COMMENT ON COLUMN file_access_permissions.revoked_at IS '软删除时间';

