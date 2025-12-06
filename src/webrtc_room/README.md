# WebRTC 房间模块

提供 WebRTC 实时音视频通信的房间管理和信令服务。

## 功能特性

- **房间创建**：登录用户可创建房间，获取房间号和密码
- **房间加入**：任何人只需房间号+密码即可加入，无需登录
- **信令转发**：支持 SDP Offer/Answer 和 ICE Candidate 转发
- **屏幕共享**：支持屏幕/窗口共享，自动重新协商
- **设备降级**：客户端自动检测摄像头/麦克风，支持纯音频或观看模式
- **TURN 自动分配**：自动为参与者分配最优 TURN 服务器
- **临时凭证**：动态生成 TURN 凭证，安全可靠

## API 端点

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/webrtc/rooms` | 创建房间 | 需要登录 |
| POST | `/api/webrtc/rooms/{room_id}/join` | 加入房间 | 无需登录 |
| GET | `/ws/webrtc/rooms/{room_id}?token=xxx` | 信令 WebSocket | Token 认证 |

## 使用流程

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WebRTC 房间使用流程                                │
└─────────────────────────────────────────────────────────────────────────────┘

┌──────────────┐                    ┌──────────────┐                    
│   创建者      │                    │   参与者      │                    
│  (已登录)     │                    │  (无需登录)   │                    
└──────┬───────┘                    └──────┬───────┘                    
       │                                   │                            
       │ 1. POST /api/webrtc/rooms         │                            
       │    创建房间                        │                            
       │                                   │                            
       │ 2. 获得 room_id + password        │                            
       │                                   │                            
       │ 3. 分享给朋友                      │                            
       │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─>│                            
       │                                   │                            
       │                                   │ 4. POST /api/webrtc/rooms/{id}/join
       │                                   │    加入房间 (密码验证)        
       │                                   │                            
       │                                   │ 5. 获得 ws_token + ice_servers
       │                                   │                            
       │ 6. WS /ws/webrtc/rooms/{id}       │ 7. WS /ws/webrtc/rooms/{id}
       │    ?token=access_token            │    ?token=ws_token          
       │                                   │                            
       │<═══════════════════════════════════>│                            
       │         8. 信令交换                 │                            
       │    (Offer/Answer/Candidate)       │                            
       │                                   │                            
       │<─────────────────────────────────>│                            
       │         9. P2P 连接建立            │                            
       │                                   │                            
```

## 模块结构

```
webrtc_room/
├── mod.rs                    # 模块入口
├── README.md                 # 本文档
├── handlers/
│   ├── mod.rs
│   ├── rooms.rs              # 创建房间 API
│   ├── join.rs               # 加入房间 API
│   ├── signaling_ws.rs       # 信令 WebSocket
│   ├── routes.rs             # 路由定义
│   └── state.rs              # 模块状态
├── models/
│   ├── mod.rs
│   ├── room.rs               # 房间数据模型
│   ├── participant.rs        # 参与者模型
│   └── signaling.rs          # 信令消息定义
└── services/
    ├── mod.rs
    ├── room_service.rs       # 房间 CRUD 服务
    ├── room_manager.rs       # 房间连接管理
    └── room_token_service.rs # 临时 Token 服务
```

## 信令消息协议

### 客户端 → 服务器

```typescript
// SDP Offer
{ "type": "offer", "to": "participant_id", "sdp": "..." }

// SDP Answer
{ "type": "answer", "to": "participant_id", "sdp": "..." }

// ICE Candidate
{ "type": "candidate", "to": "participant_id", "candidate": {...} }

// 离开房间
{ "type": "leave" }
```

### 服务器 → 客户端

```typescript
// 加入成功
{ "type": "joined", "participant_id": "xxx", "participants": [...] }

// 新参与者加入
{ "type": "peer_joined", "participant": {...} }

// 参与者离开
{ "type": "peer_left", "participant_id": "xxx" }

// 转发 Offer/Answer/Candidate
{ "type": "offer|answer|candidate", "from": "xxx", "sdp|candidate": "..." }

// 房间关闭
{ "type": "room_closed", "reason": "..." }

// 错误
{ "type": "error", "code": "xxx", "message": "..." }
```

## 配置项

房间功能使用 TURN 模块的配置：

| 环境变量 | 说明 | 默认值 |
|----------|------|--------|
| `TURN_ENABLED` | 是否启用 TURN | false |
| `TURN_AGENT_AUTH_TOKEN` | Token 签名密钥 | - |

## 安全设计

1. **房间创建**：需要登录，防止滥用
2. **房间加入**：密码验证，6位数字密码
3. **WebSocket Token**：短期有效（10分钟），HMAC-SHA256 签名
4. **TURN 凭证**：动态生成，每用户独立

## 前端协商规则

为避免 SDP Offer/Answer 冲突（Glare 问题），客户端应使用确定性规则：

- **participant_id 字典序更小的一方**发起 offer
- 这样可避免双方同时发送 offer 导致的连接问题

```javascript
// 收到 peer_joined 时
if (myParticipantId < peerParticipantId) {
  createOffer(peerId);  // 我发起
} else {
  // 等待对方发起
}
```

## 限制

- 房间最大参与人数：50 人
- 房间最长有效期：24 小时
- WebSocket Token 有效期：10 分钟
- 密码尝试：无限制（建议前端限流）
- 剪贴板复制：非 HTTPS 环境使用 `execCommand('copy')` 降级

