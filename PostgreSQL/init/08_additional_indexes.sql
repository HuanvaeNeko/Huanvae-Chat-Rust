-- ========================================
-- 补充复合索引（提升查询性能）
-- 创建时间: 2025-12-02
-- ========================================

-- ========================================
-- file_records 表复合索引
-- ========================================

-- 按用户+状态查询文件（用户文件列表）
CREATE INDEX IF NOT EXISTS idx_file_records_owner_status 
ON file_records(owner_id, status);

-- 按存储位置+状态查询（按类型统计）
CREATE INDEX IF NOT EXISTS idx_file_records_location_status 
ON file_records(storage_location, status);

-- 按哈希+状态查询（秒传检查，仅索引已完成文件）
CREATE INDEX IF NOT EXISTS idx_file_records_hash_status 
ON file_records(file_hash, status) WHERE status = 'completed';

-- 按用户+存储位置+状态复合索引（用户按类型查询文件）
CREATE INDEX IF NOT EXISTS idx_file_records_owner_location_status 
ON file_records(owner_id, storage_location, status);

-- ========================================
-- friend-messages 表复合索引
-- ========================================

-- 会话+时间复合索引（主查询：获取会话消息）
CREATE INDEX IF NOT EXISTS "idx_messages_conversation_time" 
ON "friend-messages"("conversation-uuid", "send-time" DESC);

-- 发送者+删除状态（发送者视角查询，仅索引未删除消息）
CREATE INDEX IF NOT EXISTS "idx_messages_sender_deleted" 
ON "friend-messages"("sender-id", "is-deleted-by-sender") 
WHERE "is-deleted-by-sender" = false;

-- 接收者+删除状态（接收者视角查询，仅索引未删除消息）
CREATE INDEX IF NOT EXISTS "idx_messages_receiver_deleted" 
ON "friend-messages"("receiver-id", "is-deleted-by-receiver") 
WHERE "is-deleted-by-receiver" = false;

-- 会话+发送者+删除状态复合索引（优化软删除查询）
CREATE INDEX IF NOT EXISTS "idx_messages_conv_sender_deleted"
ON "friend-messages"("conversation-uuid", "sender-id", "is-deleted-by-sender");

-- 会话+接收者+删除状态复合索引
CREATE INDEX IF NOT EXISTS "idx_messages_conv_receiver_deleted"
ON "friend-messages"("conversation-uuid", "receiver-id", "is-deleted-by-receiver");

-- ========================================
-- token-blacklist 表复合索引
-- ========================================

-- 用户+类型复合索引（按用户查询特定类型的黑名单 Token）
CREATE INDEX IF NOT EXISTS "idx_blacklist_user_type" 
ON "token-blacklist"("user-id", "token-type");

-- 用户+过期时间复合索引（清理用户黑名单时使用）
CREATE INDEX IF NOT EXISTS "idx_blacklist_user_expires"
ON "token-blacklist"("user-id", "expires-at");

-- ========================================
-- file_access_permissions 表复合索引
-- ========================================

-- 用户+访问类型复合索引（查询用户有权访问的文件）
CREATE INDEX IF NOT EXISTS idx_file_access_user_type 
ON file_access_permissions(user_id, access_type) 
WHERE revoked_at IS NULL;

-- 文件UUID+用户复合索引（验证用户对文件的访问权限）
CREATE INDEX IF NOT EXISTS idx_file_access_uuid_user
ON file_access_permissions(file_uuid, user_id)
WHERE revoked_at IS NULL;

-- ========================================
-- user-access-cache 表复合索引
-- ========================================

-- 用户+过期时间复合索引（批量拉黑时使用）
CREATE INDEX IF NOT EXISTS "idx_access_cache_user_exp"
ON "user-access-cache"("user-id", "exp")
WHERE "exp" > NOW();

-- ========================================
-- friendships 表补充索引
-- ========================================

-- 用户+好友+状态复合索引（验证好友关系）
CREATE INDEX IF NOT EXISTS idx_friendships_user_friend_status
ON friendships(user_id, friend_id, status)
WHERE status = 'active';

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '补充复合索引创建完成！';
\echo '========================================';
\echo '';
\echo '新增索引:';
\echo '';
\echo '  file_records 表:';
\echo '    - idx_file_records_owner_status';
\echo '    - idx_file_records_location_status';
\echo '    - idx_file_records_hash_status (部分索引)';
\echo '    - idx_file_records_owner_location_status';
\echo '';
\echo '  friend-messages 表:';
\echo '    - idx_messages_conversation_time';
\echo '    - idx_messages_sender_deleted (部分索引)';
\echo '    - idx_messages_receiver_deleted (部分索引)';
\echo '    - idx_messages_conv_sender_deleted';
\echo '    - idx_messages_conv_receiver_deleted';
\echo '';
\echo '  token-blacklist 表:';
\echo '    - idx_blacklist_user_type';
\echo '    - idx_blacklist_user_expires';
\echo '';
\echo '  file_access_permissions 表:';
\echo '    - idx_file_access_user_type (部分索引)';
\echo '    - idx_file_access_uuid_user (部分索引)';
\echo '';
\echo '  user-access-cache 表:';
\echo '    - idx_access_cache_user_exp (部分索引)';
\echo '';
\echo '  friendships 表:';
\echo '    - idx_friendships_user_friend_status (部分索引)';
\echo '';
\echo '========================================';

