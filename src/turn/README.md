# TURN 协调模块

分布式 TURN 服务器管理，为 WebRTC 提供 ICE 配置服务。

## 📂 目录结构

```
turn/
├── mod.rs                      # 模块入口
├── README.md                   # 本文档
├── handlers/
│   ├── mod.rs                  # Handler 模块
│   ├── routes.rs               # 路由定义
│   ├── state.rs                # TURN 状态
│   ├── ice_servers.rs          # ICE 服务器配置 API
│   └── coordinator_ws.rs       # Agent WebSocket 处理
├── models/
│   ├── mod.rs                  # 模型模块
│   ├── node.rs                 # 节点数据模型
│   └── protocol.rs             # 通信协议定义
└── services/
    ├── mod.rs                  # 服务模块
    ├── node_registry.rs        # 节点注册管理
    ├── secret_manager.rs       # 密钥管理
    ├── load_balancer.rs        # 负载均衡
    └── credential_service.rs   # 凭证签发
```

## 🎯 功能概述

### 客户端 API

| 方法 | 端点 | 说明 |
|------|------|------|
| GET | `/api/webrtc/ice-servers` | 获取 ICE 服务器配置（需认证） |

### 内部 API

| 方法 | 端点 | 说明 |
|------|------|------|
| WS | `/internal/turn-coordinator` | Agent WebSocket 连接 |

## 🔧 配置项

```env
# 是否启用 TURN 功能
TURN_ENABLED=true

# TURN 域名（用于凭证签名）
TURN_REALM=turn.example.com

# Agent 认证令牌
TURN_AGENT_AUTH_TOKEN=your-secret-token

# 凭证有效期（秒），默认 600
TURN_CREDENTIAL_TTL_SECONDS=600

# 密钥轮换间隔（小时），默认 24
TURN_SECRET_ROTATION_HOURS=24

# 节点心跳超时（秒），默认 30
TURN_HEARTBEAT_TIMEOUT_SECONDS=30
```

## 📊 架构设计

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   WebRTC 客户端  │     │   主服务器       │     │   Turn Agent    │
│   (Browser)     │     │   (Rust)        │     │   (独立部署)     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │ 1. 请求 ICE 配置      │                       │
         │ GET /api/webrtc/ice-servers                   │
         │──────────────────────>│                       │
         │                       │                       │
         │                       │ 2. Agent 注册 (WS)    │
         │                       │<──────────────────────│
         │                       │                       │
         │                       │ 3. 下发配置           │
         │                       │──────────────────────>│
         │                       │                       │
         │                       │ 4. 心跳 + 指标        │
         │                       │<──────────────────────│
         │                       │                       │
         │ 5. 返回 ICE 配置      │                       │
         │   (含动态凭证)        │                       │
         │<──────────────────────│                       │
         │                       │                       │
         │ 6. 连接 TURN 服务器   │                       │
         │───────────────────────────────────────────────>│
         │                       │                       │
```

## 🔐 凭证机制

使用 TURN REST API 标准（RFC 5766 + draft-uberti-behave-turn-rest）：

1. **用户名格式**: `timestamp:user_id`
2. **密码计算**: `base64(hmac_sha1(secret, username))`
3. **有效期**: 默认 10 分钟

```js
// 客户端使用示例
const response = await fetch('/api/webrtc/ice-servers', {
  headers: { 'Authorization': 'Bearer ' + accessToken }
});
const { ice_servers, expires_at } = await response.json();

// 配置 RTCPeerConnection
const pc = new RTCPeerConnection({
  iceServers: ice_servers.map(s => ({
    urls: s.urls,
    username: s.username,
    credential: s.credential
  }))
});
```

## 📦 数据模型

### ICE 服务器响应

```json
{
  "success": true,
  "data": {
    "ice_servers": [
      {
        "urls": ["turn:1.2.3.4:3478", "turn:1.2.3.4:3478?transport=tcp"],
        "username": "1733461200:user123",
        "credential": "base64encodedhmac",
        "credential_type": "password"
      },
      {
        "urls": ["stun:stun.l.google.com:19302"]
      }
    ],
    "expires_at": "2025-12-06T12:00:00Z"
  }
}
```

### Agent 注册消息

```json
{
  "type": "register",
  "node_id": "turn-node-01",
  "region": "cn-east",
  "public_ip": "1.2.3.4",
  "ports": {
    "listening": 3478,
    "tls": 5349,
    "min_relay": 49152,
    "max_relay": 65535
  },
  "capabilities": {
    "supports_tcp": true,
    "supports_tls": true,
    "supports_dtls": true,
    "max_bandwidth_mbps": 1000
  }
}
```

## 🚀 负载均衡策略

选择最优节点的评分规则：

| 因素 | 权重 | 说明 |
|------|------|------|
| 区域匹配 | +30 | 客户端与节点同区域 |
| 相邻区域 | +15 | 客户端与节点相邻区域 |
| CPU 负载 | -0.5/% | CPU 使用率扣分 |
| 内存负载 | -0.3/% | 内存使用率扣分 |
| 活跃会话 | -0.1/个 | 会话数扣分（最多 -20） |
| 带宽使用 | -0.3/% | 带宽使用率扣分 |

## 🔄 密钥轮换

- 默认每 24 小时自动轮换密钥
- 旧密钥在新密钥生效后继续有效一段时间（过渡期）
- 轮换时自动通知所有在线 Agent 更新配置

