# Turn Agent

TURN 服务器代理程序，连接主服务器并自动管理 Coturn 配置。

## 功能

- 自动连接主服务器获取配置
- 动态更新 Coturn 配置文件
- 定期上报节点运行指标
- 支持密钥自动轮换
- 断线自动重连

## 快速部署

### 1. 复制配置文件

```bash
cp .env.example .env
```

### 2. 编辑配置

```bash
vim .env
```

必须修改以下配置：

| 配置项 | 说明 |
|--------|------|
| `PUBLIC_IP` | 本机公网 IP（客户端通过此 IP 连接 TURN）|
| `COORDINATOR_URL` | 主服务器 WebSocket 地址 |
| `COORDINATOR_TOKEN` | Agent 认证令牌（从主服务器获取）|

### 3. 启动方式

**方式一：使用预编译二进制（推荐，更快）**

```bash
# 先在开发机编译
cargo build --release

# 然后启动容器
docker compose up -d
```

**方式二：容器内编译（较慢，需要完整 Rust 环境）**

```bash
docker compose -f compose.yaml -f compose.build.yaml up -d --build
```

**方式三：直接运行二进制（无需容器）**

```bash
# 直接运行
./target/release/turn-agent
```

## 配置说明

### 必填配置

```env
# 本机公网 IP
PUBLIC_IP=1.2.3.4

# 主服务器地址
COORDINATOR_URL=wss://api.example.com/internal/turn-coordinator

# Agent 认证令牌
COORDINATOR_TOKEN=your-token-here
```

### 可选配置

```env
# 节点标识（默认自动生成）
NODE_ID=turn-node-01

# 节点区域（用于就近分配）
REGION=cn-east

# 中继绑定 IP（云服务器 NAT 环境使用 0.0.0.0，默认 0.0.0.0）
# 如果是物理服务器可以设置为公网 IP
RELAY_IP=0.0.0.0

# TURN 端口配置
TURN_PORT=3478
TURN_TLS_PORT=5349
RELAY_MIN_PORT=49152
RELAY_MAX_PORT=65535

# 心跳间隔（秒）
HEARTBEAT_INTERVAL=5

# 日志级别
LOG_LEVEL=info
```

## 目录结构

```
turn-agent/
├── .env.example              # 配置模板
├── .env                      # 实际配置（需创建）
├── compose.yaml              # Docker Compose
├── Dockerfile                # 容器构建
├── Cargo.toml                # Rust 依赖
├── config/
│   └── turnserver.conf.template  # Coturn 配置模板
├── scripts/
│   ├── setup.sh              # 一键部署脚本
│   └── health-check.sh       # 健康检查脚本
├── src/
│   ├── main.rs               # 主程序
│   ├── config.rs             # 配置加载
│   ├── coordinator.rs        # 服务器通信
│   ├── coturn.rs             # Coturn 管理
│   ├── metrics.rs            # 指标采集
│   └── protocol.rs           # 通信协议
└── README.md
```

## 常用命令

```bash
# 查看日志
docker compose logs -f

# 查看 Agent 日志
docker compose logs -f turn-agent

# 查看 Coturn 日志
docker compose logs -f coturn

# 停止服务
docker compose down

# 重启服务
docker compose restart

# 重新构建
docker compose up -d --build
```

## 工作流程

```
1. Agent 启动 → 加载 .env 配置
2. 连接主服务器 WebSocket
3. 发送注册消息（节点信息、能力）
4. 接收配置 → 生成 turnserver.conf
5. Coturn 启动/重载
6. 定期发送心跳（包含指标）
7. 接收密钥更新 → 热更新配置
```

## 网络要求

| 端口 | 协议 | 用途 |
|------|------|------|
| 3478 | UDP/TCP | TURN 主端口 |
| 5349 | TCP | TURN TLS 端口 |
| 49152-65535 | UDP | 中继端口范围 |

确保防火墙已开放以上端口。

## 故障排查

### Agent 无法连接主服务器

1. 检查 `COORDINATOR_URL` 是否正确
2. 检查 `COORDINATOR_TOKEN` 是否有效
3. 检查网络连通性

### Coturn 未启动

1. 检查配置文件是否生成: `cat data/config/turnserver.conf`
2. 查看 Coturn 日志: `docker compose logs coturn`

### 客户端无法连接 TURN

1. 检查 `PUBLIC_IP` 是否正确
2. 检查防火墙端口是否开放
3. 检查 TURN 凭证是否有效

## 许可证

MIT

