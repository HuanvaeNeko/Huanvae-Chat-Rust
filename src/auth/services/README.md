# Services 目录

业务逻辑服务层，封装核心业务逻辑和复杂操作。

## 📂 文件说明

### `token_service.rs` (280 行)
**用途**: JWT Token 的生成、验证和刷新

**主要功能**:

#### 1. **生成 Token 对** (`generate_token_pair`)
**功能**:
- 生成 Access Token（15分钟，RSA签名）
- 生成 Refresh Token（7天，RSA签名）
- 将 Refresh Token 保存到数据库
- 返回双Token响应

**调用时机**:
- 用户登录时
- 由 `login_handler` 调用

**流程**:
```
1. 生成设备ID（基于MAC地址哈希 或 UUID）
2. 生成 Access Token
   - Claims: user_id, email, device_id, device_info, mac_address, jti, exp, iat
   - 有效期: 15分钟
   - RSA 私钥签名
3. 生成 Refresh Token
   - Claims: user_id, device_id, token_id, exp, iat
   - 有效期: 7天
   - RSA 私钥签名
4. 保存 Refresh Token 到数据库
   - 表: user-refresh-tokens
   - 记录设备信息、IP地址、过期时间
5. 返回 TokenResponse
```

---

#### 2. **生成 Access Token** (`generate_access_token`)
**功能**:
- 创建 `AccessTokenClaims`
- 使用 RSA 私钥签名
- 返回 JWT 字符串

**参数**:
- `user_id`: 用户ID
- `email`: 邮箱
- `device_id`: 设备ID
- `device_info`: 设备信息
- `mac_address`: MAC地址

**调用时机**:
- 登录时
- 刷新Token时

---

#### 3. **生成 Refresh Token** (`generate_refresh_token`)
**功能**:
- 创建 `RefreshTokenClaims`
- 使用 RSA 私钥签名
- 返回 JWT 字符串

**参数**:
- `user_id`: 用户ID
- `device_id`: 设备ID
- `token_id`: Token ID（数据库主键）

---

#### 4. **保存 Refresh Token** (`save_refresh_token`)
**功能**:
- 将 Refresh Token 信息插入数据库
- 检查同一设备是否已有Token，如有则更新

**数据库操作**:
```sql
INSERT INTO "user-refresh-tokens" (
    "token-id", "user-id", "refresh-token", "device-id",
    "device-info", "ip-address", "expires-at"
)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT ("device-id", "user-id") 
DO UPDATE SET ...
```

---

#### 5. **验证 Access Token** (`verify_access_token`)
**功能**:
- 验证 JWT 签名（RSA公钥）
- 检查过期时间
- 解析 Claims

**调用时机**:
- 中间件 `auth_guard` 中
- 每个需要认证的请求

**返回**: `AccessTokenClaims` 或错误

---

#### 6. **验证 Refresh Token** (`verify_refresh_token`)
**功能**:
- 验证 JWT 签名
- 解析 Claims

**注意**: 不检查数据库状态，只验证签名

---

#### 7. **刷新 Access Token** (`refresh_access_token`)
**功能**:
- 验证 Refresh Token
- 查询数据库确认 Token 未被撤销
- 检查过期时间
- 更新 `last-used-at`
- 生成新的 Access Token

**调用时机**:
- `POST /api/auth/refresh`
- 由 `refresh_token_handler` 调用

**流程**:
```
1. 验证 Refresh Token 签名
2. 解析 Claims (user_id, device_id, token_id)
3. 从数据库查询 Token 记录
   - 检查 is-revoked = false
4. 检查是否过期 (expires_at < now)
5. 查询用户信息 (email, device_info, mac_address)
6. 生成新的 Access Token
7. 更新 last-used-at = now
8. 返回新Token
```

---

#### 8. **查询 Refresh Token** (`get_refresh_token`)
**功能**:
- 根据 `token_id` 查询数据库
- 返回 `RefreshToken` 结构

**调用时机**:
- 刷新Token时
- 登出时查找要撤销的Token

---

**结构体**:
```rust
pub struct TokenService {
    key_manager: KeyManager,  // RSA密钥管理器
    pub(crate) db: PgPool,    // 数据库连接池
}
```

**依赖**:
- `utils/crypto.rs` - RSA密钥管理
- `models/claims.rs` - JWT Claims
- `models/refresh_token.rs` - 数据库模型

---

### `blacklist_service.rs` (115 行)
**用途**: Token 黑名单管理

**数据模型**: `BlacklistToken` (src/auth/models/blacklist.rs)

**主要功能**:

#### 1. **添加到黑名单** (`add_to_blacklist`)
**功能**:
- 将 Token 的 `jti` 添加到黑名单表
- 记录撤销原因和过期时间

**参数**:
- `jti`: JWT唯一标识
- `user_id`: 用户ID
- `token_type`: "access" 或 "refresh"
- `expires_at`: `NaiveDateTime` - Token原过期时间
- `reason`: 撤销原因（如 "用户登出", "安全事件"）

**数据库操作**:
```sql
INSERT INTO "token-blacklist" (
    "jti", "user-id", "token-type", "expires-at", "reason"
)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT ("jti") DO NOTHING
```

**调用时机**:
- 用户登出时 (logout_handler)
- 修改密码时 (未来实现)
- 检测到安全问题时 (未来实现)

**注意**: 使用 `NaiveDateTime` 保持与数据库时间类型一致

---

#### 2. **检查黑名单** (`is_blacklisted`)
**功能**:
- 查询 Token 的 `jti` 是否在黑名单中
- 返回布尔值

**查询**:
```sql
SELECT EXISTS(
    SELECT 1 FROM "token-blacklist"
    WHERE "jti" = $1 AND "expires-at" > $2
)
```

**调用时机**:
- 中间件 `auth_guard` 中
- **仅当用户的 `need-blacklist-check = true` 时才调用**
- 这是智能性能优化的关键

**时间处理**: 使用 `Utc::now().naive_utc()` 确保类型一致

---

#### 3. **清理过期黑名单** (`cleanup_expired_tokens`)
**功能**:
- 删除已过期的黑名单记录
- 减少数据库体积

**查询**:
```sql
DELETE FROM "token-blacklist"
WHERE "expires-at" < $1
```

**调用时机**:
- 定时任务（建议每小时执行一次）
- 系统维护时

**时间处理**: 使用 `Utc::now().naive_utc()` 传递当前时间

**返回**: `Result<u64, AppError>` - 删除的记录数

---

#### 4. **启用黑名单检查** (`enable_blacklist_check`)
**功能**:
- 设置用户的 `need-blacklist-check = true`
- 设置 `blacklist-check-expires-at = now + 15分钟`

**数据库操作**:
```sql
UPDATE "users"
SET "need-blacklist-check" = true,
    "blacklist-check-expires-at" = $1
WHERE "user-id" = $2
```

**调用时机**:
- 用户登出后 (logout_handler)
- 修改密码后 (未来实现)
- 远程撤销设备后 (revoke_device_handler)

**目的**: 确保在15分钟内，所有旧Token都会被黑名单检查拦截

**时间计算**: `(Utc::now() + Duration::minutes(15)).naive_utc()`

---

#### 5. **清理过期的黑名单检查标志** (`cleanup_expired_checks`)
**功能**:
- 自动禁用已过期的黑名单检查
- 将 `need-blacklist-check` 恢复为 `false`

**查询**:
```sql
UPDATE "users"
SET "need-blacklist-check" = false,
    "blacklist-check-expires-at" = NULL
WHERE "need-blacklist-check" = true
  AND "blacklist-check-expires-at" < $1
```

**调用时机**:
- 定时任务（建议每分钟执行一次）
- 自动恢复高性能模式

**时间处理**: 使用 `Utc::now().naive_utc()` 比较过期时间

**返回**: `Result<u64, AppError>` - 更新的用户数

---

#### 6. **批量拉黑用户所有 Access Token** (`blacklist_all_user_access_tokens`)
**功能**:
- 从 `user-access-cache` 读取用户所有未过期的 Access Token
- 将它们全部加入黑名单
- 自动启用黑名单检查

**参数**:
- `user_id`: 用户ID
- `reason`: 拉黑原因（如 "密码已修改"）

**查询（获取未过期 Token）**:
```sql
SELECT "jti", "exp"
FROM "user-access-cache"
WHERE "user-id" = $1 AND "exp" > NOW()
```

**调用时机**:
- 用户修改密码后（`update_password` handler）
- 确保所有旧 Access Token 立即失效

**流程**:
```
1. 查询 user-access-cache 获取所有未过期的 jti 和 exp
2. 使用批量 INSERT 一次性将所有 Token 加入黑名单（避免 N+1 查询）
3. 调用 enable_blacklist_check 启用检查窗口
4. 返回被拉黑的 Token 数量
```

**性能优化**:
- 使用 `sqlx::QueryBuilder` 的 `push_values` 方法批量插入
- 避免循环调用单条插入（N+1 问题）
- 单次数据库往返完成所有写入

**返回**: `Result<u64, AppError>` - 被拉黑的 Token 数量

**安全性**:
- 确保密码修改后，所有设备的旧 Token 立即失效
- 用户需要在所有设备重新登录

---

**结构体**:
```rust
pub struct BlacklistService {
    db: PgPool,  // 数据库连接池
}
```

**数据表**:
- `token-blacklist` - 存储被撤销的Token（包含 jti, user-id, token-type, expires-at, reason）
- `users.need-blacklist-check` - 黑名单检查开关（布尔值）
- `users.blacklist-check-expires-at` - 检查过期时间（NaiveDateTime）

**时间类型统一**: 所有时间字段均使用 `NaiveDateTime` 与数据库的 `TIMESTAMP WITHOUT TIME ZONE` 保持一致

---

## 🔒 黑名单功能详解

### 智能黑名单检查机制

黑名单功能采用**智能性能优化**设计，通过 `need-blacklist-check` 标志控制是否查询黑名单数据库。

#### 🎯 设计目标
1. **高性能**: 正常情况下跳过黑名单查询（99%的请求）
2. **安全性**: 安全事件后强制检查所有Token
3. **自动恢复**: 15分钟后自动恢复高性能模式

---

### 📊 运作流程

#### 阶段 1: 正常状态（高性能模式）
```
用户登录成功
  ↓
users.need-blacklist-check = false (默认)
  ↓
后续所有认证请求：
  ├─ auth_guard 中间件执行
  ├─ 验证 JWT 签名 ✅
  ├─ 检查 Token 过期时间 ✅
  ├─ 查询用户的 need-blacklist-check 标志
  ├─ 发现为 false
  └─ ⚡ 跳过黑名单查询 (节省 ~2-5ms)
```

**性能**: 每个请求节省一次数据库查询

---

#### 阶段 2: 安全事件触发（安全模式）
```
用户执行安全操作：
  - 主动登出 ✅
  - 修改密码（未来）
  - 远程撤销设备（未来）
  ↓
执行两个关键操作：
  ↓
1. 将当前 Access Token 加入黑名单
   └─ add_to_blacklist(jti, user_id, "access", expires_at, reason)
  ↓
2. 启用黑名单检查（15分钟）
   └─ enable_blacklist_check(user_id)
       ├─ SET need-blacklist-check = true
       └─ SET blacklist-check-expires-at = now + 15分钟
```

---

#### 阶段 3: 安全模式运行（15分钟内）
```
后续所有认证请求：
  ├─ auth_guard 中间件执行
  ├─ 验证 JWT 签名 ✅
  ├─ 检查 Token 过期时间 ✅
  ├─ 查询用户的 need-blacklist-check 标志
  ├─ 发现为 true
  ├─ 🔍 调用 is_blacklisted(jti)
  ├─    └─ 查询 token-blacklist 表
  ├─ 如果在黑名单中：
  ├─    └─ 返回 401 "Token已被撤销"
  └─ 如果不在黑名单中：
       └─ 允许请求继续
```

**作用**: 确保所有旧Token（包括尚未过期的）都被拦截

---

#### 阶段 4: 自动恢复（15分钟后）
```
定时任务执行 cleanup_expired_checks()
  ↓
查找所有过期的检查标志：
  WHERE need-blacklist-check = true
    AND blacklist-check-expires-at < NOW()
  ↓
自动恢复高性能模式：
  ├─ SET need-blacklist-check = false
  └─ SET blacklist-check-expires-at = NULL
  ↓
后续请求恢复高性能模式 ⚡
```

---

### 🕐 调用时机详解

#### 1️⃣ **add_to_blacklist** - 加入黑名单
**调用位置**: `src/auth/handlers/logout.rs:54`

```rust
// 用户登出时
let expires_at = (Utc::now() + chrono::Duration::minutes(15)).naive_utc();
blacklist_service.add_to_blacklist(
    &auth_context.claims.jti,  // JWT唯一ID
    &auth_context.user_id,
    "access",
    expires_at,
    Some("用户登出".to_string()),
).await?;
```

**为什么需要**: 
- Access Token 有效期是 15 分钟
- 用户登出后，该 Token 理论上还能用 15 分钟
- 加入黑名单后立即失效

---

#### 2️⃣ **enable_blacklist_check** - 启用检查
**调用位置**: `src/auth/handlers/logout.rs:66`

```rust
// 登出后立即启用
blacklist_service.enable_blacklist_check(&auth_context.user_id).await?;
```

**为什么需要**:
- 用户可能在多个设备登录
- 可能有多个有效的 Access Token
- 启用检查后，所有设备的旧Token都会被检查

---

#### 3️⃣ **is_blacklisted** - 检查黑名单
**调用位置**: `src/auth/middleware/auth_guard.rs`

```rust
// 中间件中的智能检查
let need_check: bool = sqlx::query_scalar(
    r#"SELECT "need-blacklist-check" FROM "users" WHERE "user-id" = $1"#
)
.bind(&claims.sub)
.fetch_one(&state.db)
.await?;

if need_check {
    // 只有启用时才查询黑名单
    let blacklist_service = BlacklistService::new(state.db.clone());
    if blacklist_service.is_blacklisted(&claims.jti).await? {
        return Err(AppError::TokenRevoked);
    }
}
```

**条件执行**: 仅当 `need-blacklist-check = true` 时

---

#### 4️⃣ **cleanup_expired_tokens** - 清理过期Token
**调用时机**: 定时任务（默认每小时，可通过 `TOKEN_CLEANUP_INTERVAL_SECONDS` 环境变量配置）

```rust
// 例如在定时任务中
let deleted = blacklist_service.cleanup_expired_tokens().await?;
tracing::info!("清理了 {} 个过期的黑名单Token", deleted);
```

**作用**: 删除已过期的黑名单记录，减少数据库体积

---

#### 5️⃣ **cleanup_expired_checks** - 清理过期检查标志
**调用时机**: 定时任务（默认每分钟，可通过 `CHECK_CLEANUP_INTERVAL_SECONDS` 环境变量配置）

```rust
// 例如在定时任务中
let updated = blacklist_service.cleanup_expired_checks().await?;
tracing::info!("恢复了 {} 个用户的高性能模式", updated);
```

**作用**: 自动将过期的检查标志恢复为 false

---

#### 6️⃣ **cleanup_expired_access_cache** - 清理过期的 Access Token 缓存
**调用时机**: 定时任务（默认每5分钟，可通过 `CACHE_CLEANUP_INTERVAL_SECONDS` 环境变量配置）

```rust
// 例如在定时任务中
let deleted = blacklist_service.cleanup_expired_access_cache().await?;
tracing::info!("清理了 {} 条过期的 user-access-cache 记录", deleted);
```

**查询**:
```sql
DELETE FROM "user-access-cache"
WHERE "exp" < $1
```

**作用**: 清理 `user-access-cache` 表中已过期的记录，减少数据库体积

---

### 🕐 定时清理任务

系统启动时会自动启动后台定时清理任务，清理间隔可通过环境变量配置：

| 清理任务 | 环境变量 | 默认值 | 说明 |
|---------|---------|-------|------|
| `token-blacklist` | `TOKEN_CLEANUP_INTERVAL_SECONDS` | 3600 (1小时) | 清理过期的黑名单记录 |
| `user-access-cache` | `CACHE_CLEANUP_INTERVAL_SECONDS` | 300 (5分钟) | 清理过期的 Token 缓存 |
| `need-blacklist-check` | `CHECK_CLEANUP_INTERVAL_SECONDS` | 60 (1分钟) | 重置过期的黑名单检查标志 |

**示例配置** (`.env` 文件):
```env
# 定时清理任务间隔配置
TOKEN_CLEANUP_INTERVAL_SECONDS=3600    # token-blacklist 清理间隔（秒）
CACHE_CLEANUP_INTERVAL_SECONDS=300     # user-access-cache 清理间隔（秒）
CHECK_CLEANUP_INTERVAL_SECONDS=60      # need-blacklist-check 清理间隔（秒）
```

**启动日志示例**:
```
🧹 定时清理任务已启动:
   - token-blacklist 清理间隔: 3600秒
   - user-access-cache 清理间隔: 300秒
   - need-blacklist-check 清理间隔: 60秒
```

---

### 📈 性能对比

| 场景 | 数据库查询次数 | 响应时间 |
|------|--------------|---------|
| **正常模式** (need-blacklist-check = false) | 1次（验证Token） | ~10ms |
| **安全模式** (need-blacklist-check = true) | 2次（验证Token + 查黑名单） | ~15ms |
| **节省** | 50% | 33% |

---

### 🔐 安全保障

1. **Access Token 泄露**: 即使Token未过期，加入黑名单后立即失效
2. **多设备攻击**: 启用检查后，所有设备的旧Token都被拦截
3. **自动恢复**: 15分钟后自动恢复，不影响长期性能
4. **过期清理**: 定时任务确保数据库不会无限增长

---

### `device_service.rs` (97 行)
**用途**: 多设备登录管理

**主要功能**:

#### 1. **列出用户设备** (`list_user_devices`)
**功能**:
- 查询用户所有未撤销的 Refresh Token
- 转换为 `Device` 结构并返回
- 标记当前设备

**查询**:
```sql
SELECT "device-id", "device-info", "ip-address", 
       "last-used-at", "created-at"
FROM "user-refresh-tokens"
WHERE "user-id" = $1 AND "is-revoked" = false
ORDER BY "last-used-at" DESC NULLS LAST
```

**调用时机**:
- `GET /api/auth/devices`
- 由 `list_devices_handler` 调用

**返回**: `Vec<Device>`

---

#### 2. **撤销设备** (`revoke_device`)
**功能**:
- 将指定设备的 Refresh Token 标记为已撤销
- 记录撤销时间和原因

**更新**:
```sql
UPDATE "user-refresh-tokens"
SET "is-revoked" = true,
    "revoked-at" = $1,
    "revoked-reason" = '远程登出'
WHERE "user-id" = $2 
  AND "device-id" = $3 
  AND "is-revoked" = false
```

**调用时机**:
- `DELETE /api/auth/devices/{device_id}`
- 由 `revoke_device_handler` 调用

**效果**: 该设备的 Refresh Token 立即失效

**完整流程** (在 `revoke_device_handler` 中):
1. **启用黑名单检查**: 调用 `enable_blacklist_check`（15分钟窗口）
2. **拉黑 Access Token**: 从缓存中读取该设备的所有 Access Token 并加入黑名单
3. **兜底处理**: 如果缓存为空且删除的是当前设备，手动拉黑当前 Token
4. **撤销 Refresh Token**: 调用 `revoke_device` 将 Refresh Token 标记为已撤销

**行为说明**:
- ⚠️ **删除其他设备**: 只撤销目标设备的 Token，不影响其他设备
- ⚠️ **删除当前设备**: 当前 Access Token 被加入黑名单，立即失效

---

#### 3. **撤销所有其他设备** (`revoke_all_other_devices`)
**功能**:
- 撤销除当前设备外的所有设备
- 用于"强制单设备登录"场景

**更新**:
```sql
UPDATE "user-refresh-tokens"
SET "is-revoked" = true,
    "revoked-at" = $1,
    "revoked-reason" = '强制单设备登录'
WHERE "user-id" = $2 
  AND "device-id" != $3
  AND "is-revoked" = false
```

**调用时机**:
- 可选功能（当前未使用）
- 可用于实现"其他设备下线"功能

---

**结构体**:
```rust
pub struct DeviceService {
    db: PgPool,  // 数据库连接池
}
```

**依赖**:
- `models/device.rs` - Device结构
- `models/refresh_token.rs` - 数据库查询

---

### `mod.rs` (9 行)
**用途**: 模块导出

**导出内容**:
```rust
pub use blacklist_service::BlacklistService;
pub use device_service::DeviceService;
pub use token_service::TokenService;
```

---

## 🔄 服务间协作

### 登录流程
```
login_handler
    ↓
TokenService::generate_token_pair
    ├─→ generate_access_token
    ├─→ generate_refresh_token
    └─→ save_refresh_token (数据库)
```

### Token刷新流程
```
refresh_token_handler
    ↓
TokenService::refresh_access_token
    ├─→ verify_refresh_token
    ├─→ get_refresh_token (数据库查询)
    ├─→ 检查is_revoked和expires_at
    ├─→ generate_access_token
    └─→ 更新last_used_at (数据库)
```

### 登出流程
```
logout_handler
    ├─→ TokenService::get_refresh_token (查询当前设备Token)
    ├─→ DeviceService::revoke_device (撤销Refresh Token)
    ├─→ BlacklistService::add_to_blacklist (加入黑名单)
    └─→ BlacklistService::enable_blacklist_check (启用检查)
```

### 认证中间件流程
```
auth_guard
    ├─→ TokenService::verify_access_token (验证签名)
    ├─→ 查询users.need_blacklist_check
    └─→ [如果启用] BlacklistService::is_blacklisted
```

### 设备管理流程
```
list_devices_handler
    ↓
DeviceService::list_user_devices
    └─→ 查询user-refresh-tokens表

revoke_device_handler
    ↓
DeviceService::revoke_device
    └─→ 更新user-refresh-tokens表
```

---

## 🎯 设计原则

**职责边界**:
- ✅ 封装业务逻辑
- ✅ 数据库操作
- ✅ 复杂的数据转换
- ✅ 跨表操作
- ❌ 不处理HTTP请求/响应（由handlers负责）
- ❌ 不直接使用Request/Response类型

**可复用性**:
- 每个服务都是独立的
- 可以被不同的handler调用
- 可以相互组合使用

**事务性**:
- 复杂操作应考虑事务
- 例如登出时的多步操作

**错误处理**:
- 统一返回 `Result<T, AppError>`（使用 `crate::common::AppError`）
- 详细的错误信息
- 便于上层处理

---

## 📊 性能考虑

### Token Service
- ✅ RSA签名（~5ms）
- ✅ 数据库写入异步
- ✅ 批量操作优化

### Blacklist Service
- ✅ 智能检查（99%跳过）
- ✅ 定时清理过期记录
- ✅ 索引优化（jti, expires-at）

### Device Service
- ✅ 按最后使用时间排序
- ✅ 仅查询未撤销设备
- ✅ 分页支持（未来）

---

## 🔧 初始化示例

```rust
// 创建服务实例
let key_manager = KeyManager::load_or_generate(
    "./keys/private_key.pem",
    "./keys/public_key.pem"
)?;

let token_service = Arc::new(TokenService::new(
    key_manager,
    db_pool.clone()
));

let blacklist_service = Arc::new(BlacklistService::new(
    db_pool.clone()
));

let device_service = Arc::new(DeviceService::new(
    db_pool.clone()
));

// 注入到handlers的State中
let login_state = LoginState {
    db: db_pool.clone(),
    token_service: token_service.clone(),
};
```

