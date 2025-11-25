# HuanVae Chat - 用户认证系统

## 安全机制概览

- 认证中间件门控：`need-blacklist-check` + `blacklist-check-expires-at`
  - 正常请求跳过黑名单查询（高性能）
  - 安全事件（登出、删除设备）开启 15 分钟窗口，窗口内查询黑名单并拦截

- 按设备拉黑（统一策略）
  - 缓存表 `user-access-cache` 记录近 15 分钟签发的 Access Token（`jti/user-id/device-id/exp/issued-at`）
  - 删除设备/登出：按 `device_id` 读取缓存，批量将命中的 `jti` 写入黑名单；缓存为空时对当前请求 `jti` 兜底拉黑

- 时间戳转换
  - 用 `chrono::DateTime::from_timestamp(exp, 0).map(|dt| dt.naive_utc())` 代替废弃的 `NaiveDateTime::from_timestamp_opt`
  - 当 `exp` 非法时，回退 `Utc::now().naive_utc()`，确保黑名单过期时间有效

- 端点与覆盖
  - 认证：`/api/auth/register`、`/api/auth/login`、`/api/auth/refresh`（公开）；`/api/auth/logout`、`/api/auth/devices`、`DELETE /api/auth/devices/{id}`（受保护）
  - 好友：`/api/friends/requests`、`/requests/approve`、`/requests/reject`、`/remove`（写，受保护）；`/requests/sent`、`/requests/pending`、`/`（读，受保护）

## 终端测试流程

1. 注册两个用户，避免重复：`u1_<timestamp>`、`u2_<timestamp>`
2. 登录 `u1` 获取 `access_token` 和 `device_id`
3. 使用 Token 发起好友请求到 `u2`
4. 删除当前设备（远程登出）
5. 使用旧 Token 再次发起好友请求（应 401）
6. 重新登录 `u1` 获取新 Token
7. 使用新 Token 再次发起好友请求（应 200）

以上流程用于验证“按设备拉黑 + 门控窗口”的即时拦截行为。

基于 Rust + Axum + PostgreSQL + JWT 的完整认证系统，支持多设备登录和智能黑名单管理。

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

### 个人资料 (Profile)
- ✅ **信息查询** - 获取完整个人信息（不含密码）
- ✅ **信息更新** - 更新邮箱、个性签名
- ✅ **密码修改** - 验证旧密码后修改
- ✅ **头像上传** - 支持 jpg/png/gif/webp，最大 5MB

### 对象存储 (Storage)
- ✅ **MinIO 集成** - S3 兼容的对象存储
- ✅ **头像存储** - 公开访问的用户头像
- ✅ **文件验证** - 类型、大小验证
- ⏳ **群文件存储** - 待实现
- ⏳ **用户文件存储** - 待实现

## 📂 项目结构

```
src/
├── auth/                  # 认证模块
│   ├── errors.rs          # 错误类型定义
│   ├── models/            # 数据模型
│   ├── utils/             # 工具函数（密钥、密码、验证）
│   ├── services/          # 业务逻辑（Token、黑名单、设备）
│   ├── middleware/        # 鉴权中间件
│   └── handlers/          # HTTP 请求处理
├── friends/               # 好友系统模块
│   ├── models/            # 请求/响应模型
│   ├── services/          # 业务逻辑（好友管理）
│   └── handlers/          # HTTP 请求处理
├── profile/               # 个人资料模块
│   ├── models/            # 请求/响应模型
│   ├── services/          # 业务逻辑（资料管理）
│   └── handlers/          # HTTP 请求处理
├── storage/               # 对象存储模块
│   ├── client.rs          # S3/MinIO 客户端
│   ├── config.rs          # 配置管理
│   └── services/          # 业务服务（头像上传）
├── main.rs                # 应用入口
└── lib.rs                 # 库导出

接口调取文档/
├── auth/                  # 认证接口文档
├── friends/               # 好友接口文档
└── profile/               # 个人资料接口文档
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

核心表：
- `users` - 用户信息
- `user-refresh-tokens` - Refresh Token 管理
- `token-blacklist` - Token 黑名单

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
# 数据库
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/huanvae_chat

# JWT 密钥
JWT_PRIVATE_KEY_PATH=./keys/rsa_private.pem
JWT_PUBLIC_KEY_PATH=./keys/rsa_public.pem

# 服务器
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# MinIO 对象存储
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin123
MINIO_BUCKET_AVATARS=avatars
MINIO_PUBLIC_URL=http://localhost:9000
MINIO_REGION=us-east-1

# 日志
RUST_LOG=info,huanvae_chat=debug
```

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

