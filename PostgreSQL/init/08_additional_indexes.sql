-- ========================================
-- 补充复合索引（提升查询性能）
-- 创建时间: 2025-12-02
-- ========================================

-- ========================================
-- file-records 表复合索引
-- ========================================

-- 按用户+状态查询文件（用户文件列表）
CREATE INDEX IF NOT EXISTS "idx-file-records-owner-status" 
ON "file-records"("owner-id", "status");

-- 按存储位置+状态查询（按类型统计）
CREATE INDEX IF NOT EXISTS "idx-file-records-location-status" 
ON "file-records"("storage-location", "status");

-- 按哈希+状态查询（秒传检查，仅索引已完成文件）
CREATE INDEX IF NOT EXISTS "idx-file-records-hash-status" 
ON "file-records"("file-hash", "status") WHERE "status" = 'completed';

-- 按用户+存储位置+状态复合索引（用户按类型查询文件）
CREATE INDEX IF NOT EXISTS "idx-file-records-owner-location-status" 
ON "file-records"("owner-id", "storage-location", "status");

-- ========================================
-- friend-messages 表复合索引
-- ========================================

-- 会话+时间复合索引（主查询：获取会话消息）
CREATE INDEX IF NOT EXISTS "idx-messages-conversation-time" 
ON "friend-messages"("conversation-uuid", "send-time" DESC);

-- 发送者+删除状态（发送者视角查询，仅索引未删除消息）
CREATE INDEX IF NOT EXISTS "idx-messages-sender-deleted" 
ON "friend-messages"("sender-id", "is-deleted-by-sender") 
WHERE "is-deleted-by-sender" = false;

-- 接收者+删除状态（接收者视角查询，仅索引未删除消息）
CREATE INDEX IF NOT EXISTS "idx-messages-receiver-deleted" 
ON "friend-messages"("receiver-id", "is-deleted-by-receiver") 
WHERE "is-deleted-by-receiver" = false;

-- 会话+发送者+删除状态复合索引（优化软删除查询）
CREATE INDEX IF NOT EXISTS "idx-messages-conv-sender-deleted"
ON "friend-messages"("conversation-uuid", "sender-id", "is-deleted-by-sender");

-- 会话+接收者+删除状态复合索引
CREATE INDEX IF NOT EXISTS "idx-messages-conv-receiver-deleted"
ON "friend-messages"("conversation-uuid", "receiver-id", "is-deleted-by-receiver");

-- ========================================
-- token-blacklist 表复合索引
-- ========================================

-- 用户+类型复合索引（按用户查询特定类型的黑名单 Token）
CREATE INDEX IF NOT EXISTS "idx-blacklist-user-type" 
ON "token-blacklist"("user-id", "token-type");

-- 用户+过期时间复合索引（清理用户黑名单时使用）
CREATE INDEX IF NOT EXISTS "idx-blacklist-user-expires"
ON "token-blacklist"("user-id", "expires-at");

-- ========================================
-- file-access-permissions 表复合索引
-- ========================================

-- 用户+访问类型复合索引（查询用户有权访问的文件）
CREATE INDEX IF NOT EXISTS "idx-file-access-user-type" 
ON "file-access-permissions"("user-id", "access-type") 
WHERE "revoked-at" IS NULL;

-- 文件UUID+用户复合索引（验证用户对文件的访问权限）
CREATE INDEX IF NOT EXISTS "idx-file-access-uuid-user"
ON "file-access-permissions"("file-uuid", "user-id")
WHERE "revoked-at" IS NULL;

-- ========================================
-- user-access-cache 表复合索引
-- ========================================

-- 用户+过期时间复合索引（批量拉黑时使用）
-- 注意：部分索引不能使用 NOW() 等 VOLATILE 函数
CREATE INDEX IF NOT EXISTS "idx-access-cache-user-exp"
ON "user-access-cache"("user-id", "exp");

-- ========================================
-- friendships 表补充索引
-- ========================================

-- 用户+好友+状态复合索引（验证好友关系）
CREATE INDEX IF NOT EXISTS "idx-friendships-user-friend-status"
ON "friendships"("user-id", "friend-id", "status")
WHERE "status" = 'active';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '补充复合索引创建完成！';
\echo '========================================';
\echo '';
\echo '新增索引:';
\echo '';
\echo '  file-records 表:';
\echo '    - idx-file-records-owner-status';
\echo '    - idx-file-records-location-status';
\echo '    - idx-file-records-hash-status (部分索引)';
\echo '    - idx-file-records-owner-location-status';
\echo '';
\echo '  friend-messages 表:';
\echo '    - idx-messages-conversation-time';
\echo '    - idx-messages-sender-deleted (部分索引)';
\echo '    - idx-messages-receiver-deleted (部分索引)';
\echo '    - idx-messages-conv-sender-deleted';
\echo '    - idx-messages-conv-receiver-deleted';
\echo '';
\echo '  token-blacklist 表:';
\echo '    - idx-blacklist-user-type';
\echo '    - idx-blacklist-user-expires';
\echo '';
\echo '  file-access-permissions 表:';
\echo '    - idx-file-access-user-type (部分索引)';
\echo '    - idx-file-access-uuid-user (部分索引)';
\echo '';
\echo '  user-access-cache 表:';
\echo '    - idx-access-cache-user-exp (部分索引)';
\echo '';
\echo '  friendships 表:';
\echo '    - idx-friendships-user-friend-status (部分索引)';
\echo '';
\echo '========================================';
