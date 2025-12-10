# WebRTC ICE 服务器配置

获取 TURN/STUN 服务器配置，用于 WebRTC 连接。

## 获取 ICE 服务器

### 请求

```http
GET /api/webrtc/ice-servers
Authorization: Bearer <access_token>
```

### 可选参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `region` | string | 客户端区域（用于就近分配），如 `cn-east` |

### 响应

```json
{
  "success": true,
  "data": {
    "ice_servers": [
      {
        "urls": [
          "turn:1.2.3.4:3478",
          "turn:1.2.3.4:3478?transport=tcp",
          "turns:1.2.3.4:5349"
        ],
        "username": "1733461200:user123",
        "credential": "K7xPm2QvNwYzH8JfL4sRcT9uBaWdE3g1",
        "credential_type": "password"
      },
      {
        "urls": [
          "stun:stun.l.google.com:19302",
          "stun:stun1.l.google.com:19302"
        ]
      }
    ],
    "expires_at": "2025-12-06T12:10:00Z"
  }
}
```

### 错误响应

```json
{
  "success": false,
  "error": {
    "code": "BAD_REQUEST",
    "message": "TURN 服务未启用"
  }
}
```

## 前端使用示例

### 获取 ICE 配置

```javascript
/**
 * 获取 ICE 服务器配置
 * @param {string} token - Access Token
 * @param {string} region - 可选，客户端区域
 * @returns {Promise<Object>}
 */
async function getIceServers(token, region = null) {
  const url = new URL('/api/webrtc/ice-servers', BASE);
  if (region) {
    url.searchParams.set('region', region);
  }

  const res = await fetch(url, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });

  if (!res.ok) {
    throw new Error(`获取 ICE 配置失败: ${res.status}`);
  }

  return res.json();
}
```

### 配置 RTCPeerConnection

```javascript
/**
 * 创建配置好的 RTCPeerConnection
 * @param {string} token - Access Token
 * @returns {Promise<RTCPeerConnection>}
 */
async function createPeerConnection(token) {
  // 1. 获取 ICE 服务器配置
  const { data } = await getIceServers(token);
  
  // 2. 转换为 RTCIceServer 格式
  const iceServers = data.ice_servers.map(server => ({
    urls: server.urls,
    username: server.username,
    credential: server.credential
  }));

  // 3. 创建 RTCPeerConnection
  const pc = new RTCPeerConnection({
    iceServers,
    iceCandidatePoolSize: 10
  });

  // 4. 设置凭证过期提醒
  const expiresAt = new Date(data.expires_at);
  const refreshTime = expiresAt.getTime() - Date.now() - 60000; // 提前1分钟
  
  if (refreshTime > 0) {
    setTimeout(() => {
      console.warn('ICE 凭证即将过期，建议刷新连接');
    }, refreshTime);
  }

  return pc;
}
```

### 完整示例：建立视频通话

```javascript
// 获取 ICE 配置并创建连接
const pc = await createPeerConnection(accessToken);

// 获取本地媒体流
const localStream = await navigator.mediaDevices.getUserMedia({
  video: true,
  audio: true
});

// 添加本地轨道
localStream.getTracks().forEach(track => {
  pc.addTrack(track, localStream);
});

// 监听远程轨道
pc.ontrack = (event) => {
  const remoteVideo = document.getElementById('remoteVideo');
  remoteVideo.srcObject = event.streams[0];
};

// 监听 ICE 候选
pc.onicecandidate = (event) => {
  if (event.candidate) {
    // 通过信令服务器发送候选给对方
    sendToSignaling({
      type: 'candidate',
      candidate: event.candidate
    });
  }
};

// 创建 Offer
const offer = await pc.createOffer();
await pc.setLocalDescription(offer);

// 发送 Offer 给对方
sendToSignaling({
  type: 'offer',
  sdp: offer.sdp
});
```

## 凭证说明

### 有效期

- 默认有效期：**10 分钟**
- 响应中的 `expires_at` 字段表示过期时间
- 建议在过期前 1 分钟刷新配置

### 凭证格式

使用 TURN REST API 标准：
- **用户名**: `timestamp:user_id`
- **密码**: `base64(hmac_sha1(secret, username))`

### 安全注意事项

1. 凭证是临时的，每次请求都会生成新凭证
2. 不要在客户端缓存凭证超过有效期
3. 凭证与用户 ID 绑定，无法跨用户使用

## 区域代码

| 代码 | 说明 |
|------|------|
| `cn-north` | 中国北方 |
| `cn-east` | 中国东部 |
| `cn-south` | 中国南方 |
| `us-west` | 美国西部 |
| `us-east` | 美国东部 |
| `eu-west` | 欧洲西部 |
| `ap-southeast` | 亚太东南 |

## 错误码

| HTTP 状态码 | 错误码 | 说明 |
|-------------|--------|------|
| 400 | BAD_REQUEST | TURN 服务未启用 |
| 401 | UNAUTHORIZED | 未登录或 Token 无效 |
| 500 | INTERNAL_ERROR | 服务器内部错误 |

