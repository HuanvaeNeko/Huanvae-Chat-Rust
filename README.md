# HuanVae Chat - 即时通讯系统

基于 Rust + Axum + PostgreSQL + MinIO + WebSocket 的完整即时通讯系统，支持好友私聊、群聊、文件传输、实时消息推送。

## ✨ 功能特性

### 认证系统 (Auth)
- ✅ **用户注册/登录** - 支持邮箱注册，密码 bcrypt 加密
- ✅ **双 Token 机制** - Access Token (15分钟) + Refresh Token (7天)
- ✅ **RSA 签名** - 使用 RSA 私钥签名，公钥验证
- ✅ **多设备登录** - 每个设备独立的 Refresh Token
- ✅ **设备管理** - 查看所有登录设备，远程撤销指定设备
- ✅ **智能黑名单** - 安全事件触发的临时黑名单检查（15分钟）
- ✅ **Token 刷新** - 自动刷新 Access Token，无缝续签

### 好友系统 (Friends)
- ✅ **好友请求** - 发送、接受、拒绝好友申请
- ✅ **好友列表** - 查看已有好友、待处理请求
- ✅ **好友管理** - 删除好友（软删除）
- ✅ **好友消息** - 私聊消息发送、获取、删除、撤回

### 群聊系统 (Groups)
- ✅ **群聊管理** - 创建/解散群聊、修改群信息
- ✅ **成员管理** - 邀请、移除、退出群聊
- ✅ **角色管理** - 群主、管理员权限
- ✅ **禁言管理** - 对成员禁言/解除禁言
- ✅ **邀请码** - 生成、使用、撤销邀请码
- ✅ **入群申请** - 申请入群、处理申请
- ✅ **群公告** - 发布、更新、删除群公告
- ✅ **群消息** - 群聊消息发送、获取、删除、撤回

### 实时通信 (WebSocket)
- ✅ **实时消息推送** - 好友/群消息实时通知
- ✅ **未读消息管理** - 未读计数、已读同步
- ✅ **心跳机制** - 连接保活、断线检测
- ✅ **已读回执** - 可配置的已读状态通知

### 个人资料 (Profile)
- ✅ **信息查询** - 获取完整个人信息（不含密码）
- ✅ **信息更新** - 更新邮箱、个性签名
- ✅ **密码修改** - 验证旧密码后修改
- ✅ **头像上传** - 支持 jpg/png/gif/webp，最大 10MB

### 对象存储 (Storage)
- ✅ **MinIO 集成** - S3 兼容的对象存储
- ✅ **头像存储** - 用户头像、群头像
- ✅ **文件验证** - 类型、大小验证
- ✅ **UUID映射去重** - 跨用户文件去重，秒传功能
- ✅ **采样哈希** - 大文件采样哈希计算，避免内存溢出
- ✅ **预签名URL** - 客户端直连MinIO，支持流式播放
- ✅ **权限管理** - 基于权限表的文件访问控制
- ✅ **好友文件** - 好友聊天文件存储
- ✅ **群文件** - 群聊文件存储

### 性能优化 (Performance)
- ✅ **时间戳分页** - 消息查询使用时间戳分页，性能提升 30-50%
- ✅ **JOIN 优化** - 群消息一次性获取发送者信息，消除 N+1 问题
- ✅ **复合索引** - 针对高频查询的数据库索引优化
- ✅ **消息归档** - 30天前消息自动归档，保持活跃表性能
- ✅ **消息缓存** - PostgreSQL 缓存热点群消息（可选）

## 📂 项目结构

```
src/
├── auth/                    # 认证模块
│   ├── models/              # 数据模型
│   ├── utils/               # 工具函数（密钥、密码、验证）
│   ├── services/            # 业务逻辑（Token、黑名单、设备）
│   ├── middleware/          # 鉴权中间件
│   └── handlers/            # HTTP 请求处理
├── friends/                 # 好友系统模块
│   ├── models/              # 请求/响应模型
│   ├── services/            # 业务逻辑（好友管理）
│   └── handlers/            # HTTP 请求处理
├── friends_messages/        # 好友消息模块
│   ├── models/              # 消息数据模型
│   ├── services/            # 消息服务（时间戳分页优化）
│   └── handlers/            # HTTP 请求处理
├── groups/                  # 群聊系统模块
│   ├── models/              # 群聊数据模型
│   ├── services/            # 群聊服务
│   └── handlers/            # HTTP 请求处理
├── group_messages/          # 群消息模块
│   ├── models/              # 群消息模型（含 JOIN 发送者信息）
│   ├── services/            # 群消息服务（JOIN 优化）
│   └── handlers/            # HTTP 请求处理
├── websocket/               # WebSocket 实时通信模块
│   ├── models/              # WS 消息协议
│   ├── services/            # 连接管理、通知服务、未读消息
│   └── handlers/            # WS 连接处理
├── profile/                 # 个人资料模块
│   ├── models/              # 请求/响应模型
│   ├── services/            # 业务逻辑（资料管理）
│   └── handlers/            # HTTP 请求处理
├── storage/                 # 对象存储模块
│   ├── client.rs            # S3/MinIO 客户端
│   └── services/            # 文件存储服务
├── common/                  # 公共模块
│   ├── errors.rs            # 统一错误类型
│   ├── response.rs          # 统一响应格式
│   └── message_archive_service.rs  # 消息归档服务
├── config.rs                # 配置管理
├── app_state.rs             # 应用状态
├── main.rs                  # 应用入口
└── lib.rs                   # 库导出

PostgreSQL/
├── init/                    # 数据库初始化脚本
│   ├── 01_core_tables.sql   # 核心表
│   ├── 02_auth_system.sql   # 认证系统表
│   ├── 03_friend_system.sql # 好友系统表
│   ├── 04_file_system.sql   # 文件系统表
│   ├── 05_indexes.sql       # 性能索引
│   ├── 06_group_system.sql  # 群聊系统表
│   └── 07_message_optimization.sql  # 消息优化（复合索引+缓存+归档）
└── 数据结构说明.md           # 数据库文档

接口调取文档/
├── auth/                    # 认证接口文档
├── friends/                 # 好友接口文档
├── messages/                # 好友消息接口文档
├── groups/                  # 群聊接口文档
├── group_messages/          # 群消息接口文档
├── storage/                 # 文件存储接口文档
└── profile/                 # 个人资料接口文档
```

## 🚀 快速开始

### 1. 安装依赖

确保已安装：
- Rust 1.91
- Podman/Docker

### 2. 启动

```bash
podman-compose up -d 
```

### 3. 构建项目

```bash
cargo build --release
```

### 4. 运行服务

```bash
cargo run
```

服务将在 `http://0.0.0.0:8080` 启动。

## 📡 API 端点

### 公开端点（无需认证）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/auth/register` | 用户注册 |
| POST | `/api/auth/login` | 用户登录 |
| POST | `/api/auth/refresh` | 刷新 Token |

### 认证端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/auth/logout` | 用户登出 |
| GET | `/api/auth/devices` | 查看所有登录设备 |
| DELETE | `/api/auth/devices/:id` | 撤销指定设备 |

### 好友端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/friends/requests` | 提交好友申请 |
| POST | `/api/friends/requests/approve` | 同意好友申请 |
| POST | `/api/friends/requests/reject` | 拒绝好友申请 |
| GET | `/api/friends/requests/sent` | 查看已发送请求 |
| GET | `/api/friends/requests/pending` | 查看待处理请求 |
| GET | `/api/friends` | 查看已有好友 |
| POST | `/api/friends/remove` | 删除好友 |

### 个人资料端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/profile` | 获取个人信息 |
| PUT | `/api/profile` | 更新邮箱/签名 |
| PUT | `/api/profile/password` | 修改密码 |
| POST | `/api/profile/avatar` | 上传头像 |

### 好友消息端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/messages` | 发送消息 |
| GET | `/api/messages` | 获取消息列表（`before_time` 时间戳分页） |
| DELETE | `/api/messages/delete` | 删除消息 |
| POST | `/api/messages/recall` | 撤回消息 |

### 群聊端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups` | 创建群聊 |
| GET | `/api/groups/my` | 获取我的群聊 |
| GET | `/api/groups/{id}` | 获取群详情 |
| PUT | `/api/groups/{id}` | 更新群信息 |
| DELETE | `/api/groups/{id}` | 解散群聊 |
| GET | `/api/groups/{id}/members` | 获取成员列表 |
| POST | `/api/groups/{id}/invite` | 邀请成员 |
| POST | `/api/groups/{id}/leave` | 退出群聊 |

### 群消息端点（需要 Token）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/group-messages` | 发送群消息 |
| GET | `/api/group-messages` | 获取消息列表（`before_time` 时间戳分页，JOIN 优化） |
| DELETE | `/api/group-messages/delete` | 删除消息（个人） |
| POST | `/api/group-messages/recall` | 撤回消息 |

### WebSocket 端点

| 路径 | 说明 |
|------|------|
| `/ws` | WebSocket 连接（需 Token 参数） |
| `/ws/status` | 连接状态检查 |

## 📖 使用示例

### 1. 用户注册

```js
await fetch('http://localhost:8080/api/auth/register', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ 'user_id': 'user123', nickname: '张三', email: 'zhangsan@example.com', password: 'password123' })
});
```

### 2. 用户登录

```js
const login = await fetch('http://localhost:8080/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ 'user_id': 'user123', password: 'password123', device_info: 'Chrome 120 on Windows 11', mac_address: '00:11:22:33:44:55' })
}).then(r => r.json());
const token = login.access_token;
```

响应：
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

### 3. 刷新 Token

```js
await fetch('http://localhost:8080/api/auth/refresh', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ refresh_token: login.refresh_token })
}).then(r => r.json());
```

### 4. 查看设备列表

```js
await fetch('http://localhost:8080/api/auth/devices', {
  headers: { 'Authorization': `Bearer ${token}` }
}).then(r => r.json());
```

### 5. 登出

```js
await fetch('http://localhost:8080/api/auth/logout', {
  method: 'POST',
  headers: { 'Authorization': `Bearer ${token}` }
});
```

## 🔐 认证流程

### 登录流程
1. 用户提交用户ID和密码
2. 验证密码（bcrypt）
3. 生成 Access Token (15分钟) 和 Refresh Token (7天)
4. Refresh Token 存入数据库，关联设备信息
5. 返回双 Token

### Token 刷新流程
1. 客户端使用 Refresh Token 请求刷新
2. 验证 Refresh Token 签名和有效期
3. 查询数据库确认 Token 未被撤销
4. 生成新的 Access Token
5. 更新 Refresh Token 的最后使用时间

### 请求认证流程
1. 客户端携带 Access Token 发送请求
2. 中间件验证 Token 签名和有效期
3. 检查用户是否需要黑名单检查（`need-blacklist-check`）
4. 如果需要，查询黑名单；否则直接放行
5. 提取用户信息，注入到请求上下文

### 登出流程
1. 撤销当前设备的 Refresh Token
2. 将当前 Access Token 加入黑名单
3. 启用用户的黑名单检查（15分钟）

## 🛡️ 安全特性

### 1. 智能黑名单检查
- **正常情况**：跳过黑名单查询，性能最优
- **安全事件**（修改密码、远程登出）：启用15分钟黑名单检查
- **自动恢复**：15分钟后自动关闭检查

### 2. 多设备管理
- 每个设备独立的 Refresh Token
- 支持查看所有登录设备
- 支持远程撤销指定设备

### 3. 密码安全
- bcrypt 哈希（cost=12）
- 密码强度验证（至少8位，包含字母和数字）

### 4. RSA 签名
- 2048位 RSA 密钥对
- 私钥签名，公钥验证
- 密钥自动生成并持久化

## 🗄️ 数据库表结构

详见 `PostgreSQL/数据结构说明.md`

核心表（24张表）：
- **用户与认证**：`users`, `user-refresh-tokens`, `token-blacklist`, `user-access-cache`, `user-storage-quotas`
- **好友系统**：`friendships`, `friend-requests`, `friend-messages`, `friend-unread-messages`
- **群聊系统**：`groups`, `group-members`, `group-join-requests`, `group-invite-codes`, `group-notices`, `group-messages`, `group-message-deletions`, `group-unread-messages`
- **文件存储**：`file-records`, `file-uuid-mapping`, `file-access-permissions`
- **消息归档**：`friend-messages-archive`, `group-messages-archive`, `group-message-cache`

性能优化索引：
- `idx-friend-messages-conv-time` - 好友消息会话+时间复合索引
- `idx-group-messages-group-time` - 群消息群ID+时间复合索引
- `idx-group-messages-sender-time` - 群消息发送者+时间索引

## 📦 依赖清单

| 依赖 | 版本 | 用途 |
|------|------|------|
| tokio | 1.48.0 | 异步运行时 |
| axum | 0.8.7 | Web 框架 + Multipart |
| tower | 0.5.2 | 中间件基础设施 |
| tower-http | 0.6.6 | HTTP 中间件 (CORS, Trace) |
| sqlx | 0.8.6 | PostgreSQL 异步客户端 |
| jsonwebtoken | 10.2.0 | JWT 签名/验证 |
| rsa | 0.9.9 | RSA 密钥管理 |
| bcrypt | 0.17.1 | 密码哈希 |
| rand | 0.8.5 | 随机数生成 |
| uuid | 1.18.1 | UUID 生成 |
| serde | 1.0.228 | 序列化/反序列化 |
| serde_json | 1.0.145 | JSON 处理 |
| chrono | 0.4.42 | 时间处理 |
| dotenvy | 0.15.7 | 环境变量加载 |
| tracing | 0.1.41 | 日志跟踪 |
| tracing-subscriber | 0.3.20 | 日志订阅器 |
| anyhow | 1.0.100 | 错误处理 |
| thiserror | 2.0.17 | 自定义错误宏 |
| validator | 0.20.0 | 数据验证 |
| aws-sdk-s3 | 1.115.0 | S3/MinIO 客户端 |
| aws-config | 1.8.11 | AWS SDK 配置 |
| aws-credential-types | 1.2.10 | AWS 凭证类型 |

## 📝 环境变量

创建 `.env` 文件：

```bash
# ========================================
# 数据库配置
# ========================================
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/huanvae_chat

# 数据库连接池配置（可选，已提供合理默认值）
DB_MAX_CONNECTIONS=20        # 最大连接数，推荐: CPU核心数 × 4
DB_MIN_CONNECTIONS=5         # 最小连接数，保持热连接
DB_ACQUIRE_TIMEOUT=30        # 获取连接超时（秒）
DB_IDLE_TIMEOUT=600          # 空闲连接超时（秒，10分钟）
DB_MAX_LIFETIME=1800         # 连接最大生命周期（秒，30分钟）

# ========================================
# JWT 密钥
# ========================================
JWT_PRIVATE_KEY_PATH=./keys/rsa_private.pem
JWT_PUBLIC_KEY_PATH=./keys/rsa_public.pem

# ========================================
# 服务器配置
# ========================================
APP_HOST=0.0.0.0
APP_PORT=8080
APP_BASE_URL=http://localhost:8080

# ========================================
# CORS 跨域配置
# ========================================
# 开发环境：允许所有来源（使用 * ）
# CORS_ALLOWED_ORIGINS=*

# 生产环境：明确指定允许的来源（多个用逗号分隔）
CORS_ALLOWED_ORIGINS=http://localhost:3000,https://yourdomain.com,https://www.yourdomain.com

# ========================================
# MinIO 对象存储
# ========================================
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin123
MINIO_BUCKET_AVATARS=avatars
MINIO_PUBLIC_URL=http://localhost:9000
MINIO_REGION=us-east-1

# ========================================
# 日志配置
# ========================================
RUST_LOG=info,sqlx=warn,hyper=info

# ========================================
# 消息配置
# ========================================
MESSAGE_RECALL_WINDOW_SECONDS=120    # 消息撤回窗口（秒），默认 2 分钟
MESSAGE_ARCHIVE_DAYS=30              # 消息归档天数，默认 30 天
MESSAGE_ARCHIVE_INTERVAL_SECONDS=86400  # 归档检查间隔，默认 24 小时
MESSAGE_CACHE_TTL_SECONDS=3600       # 消息缓存 TTL，默认 1 小时

# ========================================
# WebSocket 配置
# ========================================
WS_ENABLE_READ_RECEIPT=true          # 是否启用已读回执
WS_HEARTBEAT_INTERVAL_SECONDS=30     # 心跳间隔
WS_CLIENT_TIMEOUT_SECONDS=60         # 客户端超时
```

### 环境变量说明

#### 数据库连接池配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `DB_MAX_CONNECTIONS` | 20 | 连接池最大连接数，建议根据CPU核心数调整 |
| `DB_MIN_CONNECTIONS` | 5 | 连接池最小连接数，保持热连接提升性能 |
| `DB_ACQUIRE_TIMEOUT` | 30 | 获取连接的超时时间（秒） |
| `DB_IDLE_TIMEOUT` | 600 | 空闲连接回收时间（秒） |
| `DB_MAX_LIFETIME` | 1800 | 连接的最大生命周期（秒） |

#### CORS 跨域配置

| 变量名 | 示例值 | 说明 |
|--------|--------|------|
| `CORS_ALLOWED_ORIGINS` | `http://localhost:3000,https://yourdomain.com` | 允许的跨域来源，多个用逗号分隔。使用 `*` 允许所有来源（仅限开发环境） |

**安全建议：**
- ⚠️ 生产环境**必须**明确指定允许的域名，**禁止**使用 `*`
- ✅ 开发环境可以使用 `*` 简化调试
- ✅ 支持多个域名，用逗号分隔

#### 消息配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `MESSAGE_RECALL_WINDOW_SECONDS` | 120 | 消息撤回窗口（秒） |
| `MESSAGE_ARCHIVE_DAYS` | 30 | 消息归档天数 |
| `MESSAGE_ARCHIVE_INTERVAL_SECONDS` | 86400 | 归档检查间隔（秒） |
| `MESSAGE_CACHE_TTL_SECONDS` | 3600 | 消息缓存 TTL（秒） |

#### WebSocket 配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `WS_ENABLE_READ_RECEIPT` | true | 是否启用已读回执 |
| `WS_HEARTBEAT_INTERVAL_SECONDS` | 30 | 心跳间隔（秒） |
| `WS_CLIENT_TIMEOUT_SECONDS` | 60 | 客户端超时（秒） |

## 🔧 开发

### 运行测试
```bash
cargo test
```

### 格式化代码
```bash
cargo fmt
```

### 检查代码
```bash
cargo clippy
```

## 👨‍💻 作者

HuanVae Chat Team --欢伪

