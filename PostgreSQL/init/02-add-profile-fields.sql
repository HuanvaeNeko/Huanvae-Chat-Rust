-- ========================================
-- 添加用户个人资料字段
-- 创建时间: 2025-11-25
-- ========================================

-- 添加个性签名字段
ALTER TABLE "users" 
ADD COLUMN IF NOT EXISTS "user-signature" TEXT DEFAULT '';

-- 添加头像URL字段
ALTER TABLE "users" 
ADD COLUMN IF NOT EXISTS "user-avatar-url" TEXT DEFAULT '';

-- 创建索引以提升查询性能
CREATE INDEX IF NOT EXISTS "idx-users-avatar-url" ON "users"("user-avatar-url");

-- 显示更新信息
\echo '========================================';
\echo '用户个人资料字段添加完成！';
\echo '========================================';
\echo '';
\echo '新增字段:';
\echo '  1. user-signature    - 用户个性签名';
\echo '  2. user-avatar-url   - 用户头像URL';
\echo '';
\echo '新增索引:';
\echo '  - idx-users-avatar-url';
\echo '';
\echo '========================================';

