# WebRTC 房间接口

WebRTC 实时音视频通话房间管理接口。

## 功能特性

- **房间创建**：登录用户创建房间，获得房间号和密码
- **房间加入**：任何人使用房间号+密码加入，**无需登录**
- **信令转发**：通过 WebSocket 转发 SDP 和 ICE Candidate
- **TURN 自动分配**：自动为参与者分配最优 TURN 服务器
- **屏幕共享**：支持屏幕/窗口共享，自动重新协商
- **设备降级**：自动检测摄像头/麦克风，支持纯音频或观看模式

---

## 使用流程

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WebRTC 房间使用流程                                │
└─────────────────────────────────────────────────────────────────────────────┘

创建者（已登录）                                     参与者（无需登录）
     │                                                    │
     │ 1. POST /api/webrtc/rooms                          │
     │    创建房间                                        │
     │                                                    │
     │ 2. 获得 room_id + password                         │
     │                                                    │
     │ 3. 分享给朋友（微信/QQ等）                          │
     │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─>│
     │                                                    │
     │                                                    │ 4. POST /api/webrtc/rooms/{id}/join
     │                                                    │    输入密码加入
     │                                                    │
     │                                                    │ 5. 获得 ws_token + ice_servers
     │                                                    │
     │ 6. WS /ws/webrtc/rooms/{id}                        │ 7. WS /ws/webrtc/rooms/{id}
     │    ?token=access_token                             │    ?token=ws_token
     │                                                    │
     │<═══════════════════════════════════════════════════>│
     │              8. 信令交换                            │
     │         (Offer/Answer/Candidate)                   │
     │                                                    │
     │<───────────────────────────────────────────────────>│
     │              9. P2P 连接建立                        │
     │                                                    │
```

---

## 1. 创建房间

创建一个新的 WebRTC 房间。**需要登录**。

### 请求

```http
POST /api/webrtc/rooms
Authorization: Bearer <access_token>
Content-Type: application/json
```

### 请求体

```json
{
  "name": "小明的房间",       // 可选，房间名称
  "password": "123456",       // 可选，6位密码（不填自动生成）
  "max_participants": 10,     // 可选，最大人数（默认10，最大50）
  "expires_minutes": 120      // 可选，过期时间分钟（默认120，最大1440）
}
```

### 响应

```json
{
  "success": true,
  "data": {
    "room_id": "ABC123",
    "password": "123456",
    "name": "小明的房间",
    "max_participants": 10,
    "expires_at": "2025-12-06T14:00:00Z"
  }
}
```

### 前端示例

```js
async function createRoom() {
  const token = getToken();
  
  const res = await fetch('/api/webrtc/rooms', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      name: '语音聊天室',
      max_participants: 5
    })
  });
  
  const data = await res.json();
  
  if (data.success) {
    // 显示房间信息给用户
    alert(`房间号: ${data.data.room_id}\n密码: ${data.data.password}`);
    
    // 自动连接信令 WebSocket
    connectSignaling(data.data.room_id, token);
  }
  
  return data;
}
```

---

## 2. 加入房间

使用房间号和密码加入房间。**无需登录**。

### 请求

```http
POST /api/webrtc/rooms/{room_id}/join
Content-Type: application/json
```

### 请求体

```json
{
  "password": "123456",
  "display_name": "访客小红"
}
```

### 响应

```json
{
  "success": true,
  "data": {
    "participant_id": "p_abc12345",
    "ws_token": "eyJ...临时Token",
    "room_name": "小明的房间",
    "ice_servers": [
      {
        "urls": ["turn:1.2.3.4:3478", "turn:1.2.3.4:5349"],
        "username": "1733475000:guest_p_abc12345",
        "credential": "xxx"
      },
      {
        "urls": ["stun:stun.l.google.com:19302"]
      }
    ],
    "token_expires_at": "2025-12-06T12:10:00Z"
  }
}
```

### 前端示例

```js
async function joinRoom(roomId, password, displayName) {
  const res = await fetch(`/api/webrtc/rooms/${roomId}/join`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      password: password,
      display_name: displayName
    })
  });
  
  const data = await res.json();
  
  if (!data.success) {
    if (res.status === 401) {
      alert('密码错误');
    } else if (res.status === 404) {
      alert('房间不存在');
    } else {
      alert(data.message || '加入失败');
    }
    return null;
  }
  
  // 保存 ICE 配置
  const iceServers = data.data.ice_servers;
  
  // 连接信令 WebSocket
  connectSignaling(roomId, data.data.ws_token, iceServers);
  
  return data;
}
```

### 错误码

| HTTP 状态码 | 说明 |
|-------------|------|
| 401 | 密码错误 |
| 404 | 房间不存在 |
| 400 | 房间已过期 / 房间已满 |

---

## 3. 信令 WebSocket

建立信令连接，交换 SDP 和 ICE Candidate。

### 连接

```
WS /ws/webrtc/rooms/{room_id}?token={token}
```

- **创建者**：使用 `access_token`
- **参与者**：使用加入房间时获得的 `ws_token`

### 服务器 → 客户端消息

#### 加入成功

连接成功后，服务器发送当前房间内的参与者列表：

```json
{
  "type": "joined",
  "participant_id": "p_abc12345",
  "participants": [
    { "id": "p_xyz", "name": "小明", "is_creator": true },
    { "id": "p_123", "name": "小红", "is_creator": false }
  ]
}
```

#### 新参与者加入

```json
{
  "type": "peer_joined",
  "participant": {
    "id": "p_new",
    "name": "新用户",
    "is_creator": false
  }
}
```

#### 参与者离开

```json
{
  "type": "peer_left",
  "participant_id": "p_123"
}
```

#### SDP Offer

```json
{
  "type": "offer",
  "from": "p_xyz",
  "sdp": "v=0\r\no=- ..."
}
```

#### SDP Answer

```json
{
  "type": "answer",
  "from": "p_xyz",
  "sdp": "v=0\r\no=- ..."
}
```

#### ICE Candidate

```json
{
  "type": "candidate",
  "from": "p_xyz",
  "candidate": {
    "candidate": "candidate:...",
    "sdpMLineIndex": 0,
    "sdpMid": "0"
  }
}
```

#### 房间关闭

```json
{
  "type": "room_closed",
  "reason": "创建者关闭了房间"
}
```

#### 错误

```json
{
  "type": "error",
  "code": "invalid_message",
  "message": "消息格式无效"
}
```

### 客户端 → 服务器消息

#### 发送 SDP Offer

```json
{
  "type": "offer",
  "to": "p_target_id",
  "sdp": "v=0\r\no=- ..."
}
```

#### 发送 SDP Answer

```json
{
  "type": "answer",
  "to": "p_target_id",
  "sdp": "v=0\r\no=- ..."
}
```

#### 发送 ICE Candidate

```json
{
  "type": "candidate",
  "to": "p_target_id",
  "candidate": {
    "candidate": "candidate:...",
    "sdpMLineIndex": 0,
    "sdpMid": "0"
  }
}
```

#### 离开房间

```json
{
  "type": "leave"
}
```

---

## 4. 完整前端示例

```js
class WebRTCRoom {
  constructor() {
    this.ws = null;
    this.peerConnections = {}; // participant_id -> RTCPeerConnection
    this.localStream = null;
    this.iceServers = [];
    this.myId = null;
  }

  // 连接信令服务器
  async connect(roomId, token, iceServers = []) {
    this.iceServers = iceServers;
    
    // 建立 WebSocket 连接
    const wsUrl = `wss://api.example.com/ws/webrtc/rooms/${roomId}?token=${token}`;
    this.ws = new WebSocket(wsUrl);
    
    this.ws.onmessage = (event) => this.handleMessage(JSON.parse(event.data));
    this.ws.onclose = () => console.log('信令连接已断开');
    this.ws.onerror = (err) => console.error('信令错误:', err);
  }

  // 处理服务器消息
  async handleMessage(msg) {
    switch (msg.type) {
      case 'joined':
        this.myId = msg.participant_id;
        console.log('已加入房间，当前参与者:', msg.participants);
        // 向每个现有参与者发起连接
        for (const p of msg.participants) {
          await this.createOffer(p.id);
        }
        break;
        
      case 'peer_joined':
        console.log('新参与者加入:', msg.participant);
        // 等待对方发起 Offer
        break;
        
      case 'peer_left':
        console.log('参与者离开:', msg.participant_id);
        this.closePeerConnection(msg.participant_id);
        break;
        
      case 'offer':
        await this.handleOffer(msg.from, msg.sdp);
        break;
        
      case 'answer':
        await this.handleAnswer(msg.from, msg.sdp);
        break;
        
      case 'candidate':
        await this.handleCandidate(msg.from, msg.candidate);
        break;
        
      case 'room_closed':
        alert('房间已关闭: ' + msg.reason);
        this.disconnect();
        break;
        
      case 'error':
        console.error('服务器错误:', msg.code, msg.message);
        break;
    }
  }

  // 创建 PeerConnection
  createPeerConnection(peerId) {
    const pc = new RTCPeerConnection({
      iceServers: this.iceServers
    });
    
    // 添加本地流
    if (this.localStream) {
      this.localStream.getTracks().forEach(track => {
        pc.addTrack(track, this.localStream);
      });
    }
    
    // ICE Candidate 事件
    pc.onicecandidate = (event) => {
      if (event.candidate) {
        this.send({
          type: 'candidate',
          to: peerId,
          candidate: event.candidate
        });
      }
    };
    
    // 接收远程流
    pc.ontrack = (event) => {
      console.log('收到远程流:', peerId);
      // 显示远程视频
      const video = document.getElementById(`video-${peerId}`);
      if (video) {
        video.srcObject = event.streams[0];
      }
    };
    
    this.peerConnections[peerId] = pc;
    return pc;
  }

  // 发起 Offer
  async createOffer(peerId) {
    const pc = this.createPeerConnection(peerId);
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    
    this.send({
      type: 'offer',
      to: peerId,
      sdp: offer.sdp
    });
  }

  // 处理 Offer
  async handleOffer(peerId, sdp) {
    const pc = this.createPeerConnection(peerId);
    await pc.setRemoteDescription({ type: 'offer', sdp });
    
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);
    
    this.send({
      type: 'answer',
      to: peerId,
      sdp: answer.sdp
    });
  }

  // 处理 Answer
  async handleAnswer(peerId, sdp) {
    const pc = this.peerConnections[peerId];
    if (pc) {
      await pc.setRemoteDescription({ type: 'answer', sdp });
    }
  }

  // 处理 ICE Candidate
  async handleCandidate(peerId, candidate) {
    const pc = this.peerConnections[peerId];
    if (pc) {
      await pc.addIceCandidate(new RTCIceCandidate(candidate));
    }
  }

  // 发送消息
  send(msg) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  // 关闭连接
  closePeerConnection(peerId) {
    const pc = this.peerConnections[peerId];
    if (pc) {
      pc.close();
      delete this.peerConnections[peerId];
    }
  }

  // 断开连接
  disconnect() {
    this.send({ type: 'leave' });
    
    Object.keys(this.peerConnections).forEach(id => {
      this.closePeerConnection(id);
    });
    
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  // 获取本地媒体流
  async getLocalStream() {
    this.localStream = await navigator.mediaDevices.getUserMedia({
      video: true,
      audio: true
    });
    return this.localStream;
  }
}

// 使用示例
const room = new WebRTCRoom();

// 创建者
async function startAsCreator() {
  await room.getLocalStream();
  const { data } = await createRoom();
  await room.connect(data.room_id, getToken(), []);
}

// 参与者
async function startAsGuest(roomId, password, name) {
  await room.getLocalStream();
  const { data } = await joinRoom(roomId, password, name);
  await room.connect(roomId, data.ws_token, data.ice_servers);
}
```

---

---

## 5. 屏幕共享

### 开始屏幕共享

使用 `getDisplayMedia` API 获取屏幕流：

```js
async function startScreenShare() {
  const screenStream = await navigator.mediaDevices.getDisplayMedia({
    video: { cursor: 'always' },
    audio: true  // 可选：共享系统音频
  });
  
  const videoTrack = screenStream.getVideoTracks()[0];
  
  // 对每个 PeerConnection 添加或替换视频轨道
  for (const [peerId, pc] of Object.entries(peerConnections)) {
    const sender = pc.getSenders().find(s => s.track?.kind === 'video');
    
    if (sender) {
      // 已有视频轨道，直接替换
      await sender.replaceTrack(videoTrack);
    } else {
      // 没有视频轨道（如无摄像头），添加并重新协商
      pc.addTrack(videoTrack, screenStream);
      
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      sendSignaling({ type: 'offer', to: peerId, sdp: pc.localDescription.sdp });
    }
  }
  
  // 监听停止共享
  videoTrack.onended = () => stopScreenShare();
}
```

### 设备检测与降级

```js
async function initMedia() {
  // 先枚举设备
  const devices = await navigator.mediaDevices.enumerateDevices();
  const hasVideo = devices.some(d => d.kind === 'videoinput');
  const hasAudio = devices.some(d => d.kind === 'audioinput');
  
  // 根据可用设备构建约束
  let constraints = {};
  if (hasVideo) constraints.video = true;
  if (hasAudio) constraints.audio = true;
  
  if (hasVideo || hasAudio) {
    try {
      localStream = await navigator.mediaDevices.getUserMedia(constraints);
    } catch (e) {
      // 降级：仅音频
      if (hasAudio) {
        localStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      }
    }
  }
  
  // 即使没有媒体设备也可以加入（观看模式）
}
```

---

## 6. Offer/Answer 协商规则

为避免双方同时发起 Offer 导致的冲突（Glare 问题），使用确定性规则：

```js
// participant_id 字典序更小的一方发起 offer
if (myParticipantId < peerParticipantId) {
  // 我发起 offer
  createPeerConnection(peerId, true);
} else {
  // 我等待对方 offer
  createPeerConnection(peerId, false);
}
```

### 连接建立流程

```
用户 A (p_aaa)                      用户 B (p_bbb)
     │                                   │
     │ 收到 peer_joined (p_bbb)          │
     │                                   │
     │ p_aaa < p_bbb → A 发起 offer      │
     │ ───────────── offer ────────────→ │
     │                                   │
     │ ←──────────── answer ──────────── │
     │                                   │
     │ ←───────── ICE candidates ──────→ │
     │                                   │
     └─────────── 连接建立 ──────────────┘
```

---

## 注意事项

1. **Token 有效期**：`ws_token` 有效期 10 分钟，需在有效期内建立 WebSocket 连接
2. **房间有效期**：房间默认 2 小时后过期，最长 24 小时
3. **人数限制**：单个房间最多 50 人
4. **HTTPS**：生产环境必须使用 HTTPS/WSS（屏幕共享也需要 HTTPS）
5. **媒体权限**：使用 WebRTC 前需获取用户摄像头/麦克风权限
6. **屏幕共享重协商**：无摄像头时开启屏幕共享需要重新协商（自动发送 offer）
7. **剪贴板兼容**：非 HTTPS 环境使用 `execCommand('copy')` 降级方案
8. **Offer 冲突避免**：使用 participant_id 比较规则，ID 更小的一方发起 offer

