-- ========================================
-- HuanVae Chat 认证系统表结构
-- 创建时间: 2025-12-03
-- 包含: user-refresh-tokens, token-blacklist, user-access-cache
-- ========================================

-- ========================================
-- 用户刷新Token管理表 (user-refresh-tokens)
-- 支持多设备登录，每个设备独立的Refresh Token
-- ========================================
CREATE TABLE IF NOT EXISTS "user-refresh-tokens" (
    "token-id" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "refresh-token" TEXT NOT NULL UNIQUE,
    "device-id" TEXT NOT NULL,
    "device-info" TEXT,  -- JSON格式: {"device_type":"mobile","os":"Android 14",...}
    "ip-address" TEXT,
    "created-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "expires-at" TIMESTAMP NOT NULL,
    "last-used-at" TIMESTAMP,
    "is-revoked" BOOLEAN DEFAULT false,
    "revoked-at" TIMESTAMP,
    "revoked-reason" TEXT,
    
    -- 外键约束
    CONSTRAINT "fk-refresh-tokens-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- Token黑名单表 (token-blacklist)
-- 用于紧急撤销Access Token（15分钟有效期内需要立即失效）
-- ========================================
CREATE TABLE IF NOT EXISTS "token-blacklist" (
    "jti" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "token-type" TEXT NOT NULL,  -- "access" 或 "refresh"
    "expires-at" TIMESTAMP NOT NULL,
    "blacklisted-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "reason" TEXT,
    
    -- 外键约束
    CONSTRAINT "fk-blacklist-user"
        FOREIGN KEY ("user-id") 
        REFERENCES "users"("user-id") 
        ON DELETE CASCADE
);

-- ========================================
-- 用户Access Token缓存表 (user-access-cache)
-- 记录近15分钟签发的Access Token用于按设备精准拉黑
-- ========================================
CREATE TABLE IF NOT EXISTS "user-access-cache" (
    "jti" TEXT PRIMARY KEY,
    "user-id" TEXT NOT NULL,
    "device-id" TEXT NOT NULL,
    "exp" TIMESTAMP NOT NULL,
    "issued-at" TIMESTAMP NOT NULL,
    "inserted-at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "fk-access-cache-user"
        FOREIGN KEY ("user-id")
        REFERENCES "users"("user-id")
        ON DELETE CASCADE
);

-- ========================================
-- 创建索引
-- ========================================

-- Refresh Token表索引
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-user-id" ON "user-refresh-tokens"("user-id");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-token" ON "user-refresh-tokens"("refresh-token");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-expires" ON "user-refresh-tokens"("expires-at");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-device" ON "user-refresh-tokens"("device-id");
CREATE INDEX IF NOT EXISTS "idx-refresh-tokens-revoked" ON "user-refresh-tokens"("is-revoked") WHERE "is-revoked" = false;

-- Token黑名单表索引
CREATE INDEX IF NOT EXISTS "idx-blacklist-user-id" ON "token-blacklist"("user-id");
CREATE INDEX IF NOT EXISTS "idx-blacklist-expires" ON "token-blacklist"("expires-at");
CREATE INDEX IF NOT EXISTS "idx-blacklist-type" ON "token-blacklist"("token-type");

-- Access Token缓存表索引
CREATE INDEX IF NOT EXISTS "idx-access-cache-user-device" ON "user-access-cache"("user-id", "device-id");
CREATE INDEX IF NOT EXISTS "idx-access-cache-exp" ON "user-access-cache"("exp");

-- ========================================
-- 显示创建信息
-- ========================================
\echo '========================================';
\echo '认证系统表创建完成！';
\echo '========================================';
\echo '';
\echo '创建的表:';
\echo '  1. user-refresh-tokens - 刷新Token管理表（多设备）';
\echo '  2. token-blacklist     - Token黑名单表';
\echo '  3. user-access-cache   - Access Token缓存表';
\echo '';
\echo '创建的索引:';
\echo '  user-refresh-tokens: 5个索引';
\echo '  token-blacklist:     3个索引';
\echo '  user-access-cache:   2个索引';
\echo '';
\echo '========================================';

