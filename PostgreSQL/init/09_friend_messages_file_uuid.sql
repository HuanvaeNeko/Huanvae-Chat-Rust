-- ========================================
-- 为好友消息表添加 file-uuid 字段
-- 创建时间: 2025-12-03
-- ========================================

-- 添加 file-uuid 字段
ALTER TABLE "friend-messages" ADD COLUMN IF NOT EXISTS "file-uuid" VARCHAR(36);

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx-friend-messages-file-uuid" 
    ON "friend-messages"("file-uuid") 
    WHERE "file-uuid" IS NOT NULL;

-- ========================================
-- 显示更新信息
-- ========================================
\echo '========================================';
\echo '好友消息表 file-uuid 字段添加完成！';
\echo '========================================';
\echo '';
\echo '新增字段:';
\echo '  - file-uuid VARCHAR(36) - 文件UUID，关联 file-uuid-mapping 表';
\echo '';
\echo '新增索引:';
\echo '  - idx-friend-messages-file-uuid';
\echo '';
\echo '========================================';

