# HuanVae Chat - 用户认证系统

基于 Rust + Axum + PostgreSQL + JWT 的完整认证系统，支持多设备登录和智能黑名单管理。

## ✨ 功能特性

- ✅ **用户注册/登录** - 支持邮箱注册，密码 bcrypt 加密
- ✅ **双 Token 机制** - Access Token (15分钟) + Refresh Token (7天)
- ✅ **RSA 签名** - 使用 RSA 私钥签名，公钥验证
- ✅ **多设备登录** - 每个设备独立的 Refresh Token
- ✅ **设备管理** - 查看所有登录设备，远程撤销指定设备
- ✅ **智能黑名单** - 安全事件触发的临时黑名单检查（15分钟）
- ✅ **Token 刷新** - 自动刷新 Access Token，无缝续签

## 📂 项目结构

```
src/auth/
├── errors.rs              # 错误类型定义
├── models/                # 数据模型
│   ├── user.rs            # 用户模型
│   ├── claims.rs          # JWT Claims
│   ├── refresh_token.rs   # Refresh Token 模型
│   └── device.rs          # 设备信息模型
├── utils/                 # 工具函数
│   ├── crypto.rs          # RSA 密钥管理
│   ├── password.rs        # 密码哈希
│   └── validator.rs       # 输入验证
├── services/              # 业务逻辑
│   ├── token_service.rs   # Token 生成/验证/刷新
│   ├── blacklist_service.rs # 黑名单管理
│   └── device_service.rs  # 设备管理
├── middleware/            # 中间件
│   └── auth_guard.rs      # 鉴权中间件
└── handlers/              # HTTP 请求处理
    ├── register.rs        # 注册
    ├── login.rs           # 登录
    ├── refresh_token.rs   # 刷新 Token
    ├── logout.rs          # 登出
    └── revoke_device.rs   # 设备管理
```

## 🚀 快速开始

### 1. 安装依赖

确保已安装：
- Rust 1.75+
- PostgreSQL 18.1+
- Podman/Docker

### 2. 启动

```bash
cd /home/huanwei/new-huanvae-chat
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

### 需要认证的端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/auth/logout` | 用户登出 |
| GET | `/api/auth/devices` | 查看所有登录设备 |
| DELETE | `/api/auth/devices/:id` | 撤销指定设备 |

## 📖 使用示例

### 1. 用户注册

```bash
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "nickname": "张三",
    "email": "zhangsan@example.com",
    "password": "password123"
  }'
```

### 2. 用户登录

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "password": "password123",
    "device_info": "Chrome 120 on Windows 11",
    "mac_address": "00:11:22:33:44:55"
  }'
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

```bash
curl -X POST http://localhost:8080/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJ..."
  }'
```

### 4. 查看设备列表

```bash
curl -X GET http://localhost:8080/api/auth/devices \
  -H "Authorization: Bearer eyJ..."
```

### 5. 登出

```bash
curl -X POST http://localhost:8080/api/auth/logout \
  -H "Authorization: Bearer eyJ..."
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
| axum | 0.8.7 | Web 框架 |
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

## 📝 环境变量

创建 `.env` 文件：

```bash
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/huanvae_chat
JWT_PRIVATE_KEY_PATH=./keys/private_key.pem
JWT_PUBLIC_KEY_PATH=./keys/public_key.pem
APP_PORT=8080
RUST_LOG=info,sqlx=warn
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

## 📄 许可证

MIT License

## 👨‍💻 作者

HuanVae Chat Team

