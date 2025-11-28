-- 文件记录表
CREATE TABLE IF NOT EXISTS file_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_key VARCHAR(500) NOT NULL UNIQUE,
    file_url TEXT,
    file_type VARCHAR(50) NOT NULL,
    storage_location VARCHAR(50) NOT NULL,
    preview_support VARCHAR(20) NOT NULL DEFAULT 'download_only',
    
    -- 所有者信息
    owner_id VARCHAR(255) NOT NULL,
    related_id VARCHAR(255),
    
    -- 文件信息
    file_size BIGINT NOT NULL,
    actual_size BIGINT,
    content_type VARCHAR(100) NOT NULL,
    
    -- 哈希值（去重和完整性校验）
    file_hash VARCHAR(64) NOT NULL,
    
    -- 物理文件引用（用于去重）
    physical_file_key VARCHAR(500),
    
    -- 上传凭证
    upload_token VARCHAR(128),
    multipart_upload_id VARCHAR(200),
    
    -- 状态管理
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_file_hash ON file_records(file_hash);
CREATE INDEX IF NOT EXISTS idx_owner_id ON file_records(owner_id);
CREATE INDEX IF NOT EXISTS idx_status_expires ON file_records(status, expires_at);
CREATE INDEX IF NOT EXISTS idx_upload_token ON file_records(upload_token) WHERE upload_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_related_id ON file_records(related_id) WHERE related_id IS NOT NULL;

-- 文件引用表（秒传时使用）
CREATE TABLE IF NOT EXISTS file_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_file_key VARCHAR(500) NOT NULL,
    owner_id VARCHAR(255) NOT NULL,
    file_type VARCHAR(50) NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_ref_owner_file ON file_references(owner_id, original_file_key);
CREATE INDEX IF NOT EXISTS idx_ref_hash ON file_references(file_hash);

-- 用户存储配额表
CREATE TABLE IF NOT EXISTS user_storage_quotas (
    user_id VARCHAR(255) PRIMARY KEY,
    total_quota BIGINT NOT NULL DEFAULT 10737418240, -- 10GB默认
    used_space BIGINT NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 触发器：文件完成时更新配额
CREATE OR REPLACE FUNCTION update_user_quota()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'completed' AND OLD.status != 'completed' THEN
        INSERT INTO user_storage_quotas (user_id, used_space, file_count)
        VALUES (NEW.owner_id, NEW.file_size, 1)
        ON CONFLICT (user_id) DO UPDATE
        SET used_space = user_storage_quotas.used_space + NEW.file_size,
            file_count = user_storage_quotas.file_count + 1,
            updated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_quota ON file_records;
CREATE TRIGGER trigger_update_quota
AFTER UPDATE ON file_records
FOR EACH ROW
EXECUTE FUNCTION update_user_quota();

-- 添加注释
COMMENT ON TABLE file_records IS '文件记录表，存储所有上传文件的元数据';
COMMENT ON COLUMN file_records.file_hash IS 'SHA-256哈希值，用于去重和完整性校验';
COMMENT ON COLUMN file_records.upload_token IS '一次性上传Token，上传完成后清空';
COMMENT ON COLUMN file_records.preview_support IS '预览支持：inline_preview 或 download_only';
COMMENT ON COLUMN file_records.status IS '状态：pending（待上传）、completed（已完成）、failed（失败）';

COMMENT ON TABLE file_references IS '文件引用表，用于秒传功能';
COMMENT ON TABLE user_storage_quotas IS '用户存储配额表';

