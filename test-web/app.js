/**
 * HuanVae Chat - 前端应用
 * 完整功能：认证、好友、消息、群聊、文件存储
 */

// ==========================================
// 配置和全局状态
// ==========================================

// 根据当前域名自动选择 API 地址
const BASE_URL = (() => {
  const hostname = window.location.hostname;
  const protocol = window.location.protocol;
  
  // web.xxx.cn -> api.xxx.cn
  if (hostname.startsWith('web.')) {
    return `${protocol}//api.${hostname.slice(4)}`;
  }
  
  // 本地 IP 访问（如 192.168.x.x）或 localhost -> 去掉端口，使用 80
  return `${protocol}//${hostname}`;
})();

console.log('📡 API 地址:', BASE_URL);

const state = {
  accessToken: localStorage.getItem('accessToken') || '',
  refreshToken: localStorage.getItem('refreshToken') || '',
  currentUser: JSON.parse(localStorage.getItem('currentUser') || 'null'),
  
  // 当前聊天
  currentChat: null,  // { type: 'friend' | 'group', id: string, name: string }
  
  // 缓存
  friends: [],
  groups: [],
  conversations: [],
  messages: {},  // { chatId: [...messages] }
  groupMembers: {},  // { groupId: [...members] }
  
  // WebSocket 相关
  ws: null,           // WebSocket 实例
  wsConnected: false, // 连接状态
  wsReconnectTimer: null, // 重连定时器
  wsPingInterval: null,   // 心跳定时器
  unreadSummary: null,    // 未读消息摘要
};

// ==========================================
// 工具函数
// ==========================================

// API 请求封装（带自动 Token 刷新）
async function api(path, { method = 'GET', body, formData, token = state.accessToken, _retried = false } = {}) {
  const headers = {};
  
  if (body) {
    headers['Content-Type'] = 'application/json';
  }
  
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  
  const options = { method, headers };
  
  if (formData) {
    delete headers['Content-Type'];
    options.body = formData;
  } else if (body) {
    options.body = JSON.stringify(body);
  }
  
  const res = await fetch(`${BASE_URL}${path}`, options);
  
  const contentType = res.headers.get('content-type') || '';
  let data;
  
  if (contentType.includes('application/json')) {
    data = await res.json().catch(() => ({}));
  } else {
    const text = await res.text().catch(() => '');
    data = text ? { message: text } : {};
  }
  
  // 401 错误且未重试过 → 尝试刷新 Token 并重试
  if (res.status === 401 && !_retried && state.refreshToken) {
    console.log('🔄 收到 401，尝试刷新 Token 后重试...');
    const refreshed = await refreshTokenRequest();
    if (refreshed) {
      // 用新 Token 重试请求
      return api(path, { method, body, formData, token: state.accessToken, _retried: true });
    }
  }
  
  if (!res.ok) {
    throw new Error(data.error || data.message || `请求失败: ${res.status}`);
  }
  
  return data;
}

// 解码 JWT
function decodeJwt(token) {
  try {
    const [, payload] = token.split('.');
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(json);
  } catch {
    return null;
  }
}

// 格式化时间
function formatTime(isoStr) {
  if (!isoStr) return '';
  const date = new Date(isoStr);
  const now = new Date();
  const diff = now - date;
  
  if (diff < 60000) return '刚刚';
  if (diff < 3600000) return Math.floor(diff / 60000) + '分钟前';
  if (diff < 86400000 && date.getDate() === now.getDate()) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  }
  if (diff < 172800000) return '昨天';
  return date.toLocaleDateString('zh-CN', { month: 'numeric', day: 'numeric' });
}

// 格式化文件大小
function formatSize(bytes) {
  if (!bytes) return '0 B';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB';
  return (bytes / 1024 / 1024 / 1024).toFixed(1) + ' GB';
}

// 计算文件 SHA-256 哈希（仅基于文件内容，不包含元数据，确保相同内容产生相同哈希）
async function calculateSHA256(file) {
  const SAMPLE_SIZE = 10 * 1024 * 1024; // 10MB
  
  let dataToHash;
  
  if (file.size <= SAMPLE_SIZE * 3) {
    // 小文件：完整读取内容计算哈希
    dataToHash = await file.arrayBuffer();
  } else {
    // 大文件：采样计算（头部 + 中部 + 尾部 + 文件大小）
    // 文件大小作为额外信息确保不同大小的文件产生不同哈希
    const sizeBuffer = new TextEncoder().encode(`|size:${file.size}|`);
    
    const chunks = [];
    // 头部 10MB
    chunks.push(new Uint8Array(await file.slice(0, SAMPLE_SIZE).arrayBuffer()));
    // 中部 10MB
    const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
    chunks.push(new Uint8Array(await file.slice(middleStart, middleStart + SAMPLE_SIZE).arrayBuffer()));
    // 尾部 10MB
    chunks.push(new Uint8Array(await file.slice(file.size - SAMPLE_SIZE, file.size).arrayBuffer()));
    
    const totalLength = sizeBuffer.length + chunks.reduce((sum, c) => sum + c.length, 0);
    dataToHash = new Uint8Array(totalLength);
    let offset = 0;
    dataToHash.set(sizeBuffer, offset);
    offset += sizeBuffer.length;
    for (const chunk of chunks) {
      dataToHash.set(chunk, offset);
      offset += chunk.length;
    }
  }
  
  const hashBuffer = await crypto.subtle.digest('SHA-256', dataToHash);
  return Array.from(new Uint8Array(hashBuffer)).map(b => b.toString(16).padStart(2, '0')).join('');
}

// Toast 提示
function showToast(message, type = 'info') {
  const container = document.getElementById('toastContainer');
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  container.appendChild(toast);
  
  setTimeout(() => toast.remove(), 3000);
}

// 打开/关闭模态框
function openModal(id) {
  document.getElementById(id).style.display = 'flex';
}

function closeModal(id) {
  document.getElementById(id).style.display = 'none';
}

// ==========================================
// WebRTC 视频房间
// ==========================================

// 显示创建房间模态框
function showCreateRoomModal() {
  if (!state.accessToken) {
    showToast('请先登录', 'error');
    openModal('authModal');
    return;
  }
  openModal('createRoomModal');
}

// 创建视频房间
async function createVideoRoom(event) {
  event.preventDefault();
  
  const name = document.getElementById('roomName').value.trim();
  const password = document.getElementById('roomPassword').value.trim();
  const maxParticipants = parseInt(document.getElementById('roomMaxParticipants').value);
  const expiresMinutes = parseInt(document.getElementById('roomExpires').value);
  
  try {
    const response = await api('/api/webrtc/rooms', {
      method: 'POST',
      body: {
        name: name || undefined,
        password: password || undefined,
        max_participants: maxParticipants,
        expires_minutes: expiresMinutes  // 修正字段名
      }
    });
    
    console.log('📹 创建房间响应:', response);  // 调试日志
    
    closeModal('createRoomModal');
    showToast('房间创建成功！', 'success');
    
    // 显示房间信息（后端返回 ApiResponse 格式，实际数据在 response.data 中）
    const roomData = response.data || response;
    console.log('📹 房间数据:', roomData);  // 调试日志
    showRoomCreatedInfo(roomData);
    
  } catch (err) {
    showToast(err.message || '创建房间失败', 'error');
  }
}

// 显示房间创建成功信息
function showRoomCreatedInfo(roomData) {
  const roomId = roomData.room_id || roomData.id;
  const password = roomData.password || '';
  
  // 创建信息弹窗
  const infoHtml = `
    <div class="room-created-info">
      <h4>🎉 房间创建成功</h4>
      <div class="info-item">
        <label>房间号：</label>
        <span class="room-id">${roomId}</span>
        <button class="btn-small" onclick="copyToClipboard('${roomId}')">复制</button>
      </div>
      ${password ? `
      <div class="info-item">
        <label>密码：</label>
        <span class="room-password">${password}</span>
        <button class="btn-small" onclick="copyToClipboard('${password}')">复制</button>
      </div>
      ` : ''}
      <div class="info-actions">
        <button class="btn-primary" onclick="goToRoom('${roomId}')">进入房间</button>
      </div>
    </div>
  `;
  
  // 使用 toast 或 alert 显示
  if (confirm(`房间创建成功！\n房间号: ${roomId}\n${password ? '密码: ' + password : ''}\n\n是否立即进入房间？`)) {
    goToRoom(roomId);
  }
}

// 跳转到房间页面
function goToRoom(roomId) {
  window.open(`room.html?room=${roomId}`, '_blank');
}

// 复制到剪贴板
function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    showToast('已复制到剪贴板', 'success');
  }).catch(() => {
    showToast('复制失败', 'error');
  });
}

// 显示加入房间模态框
function showJoinRoomModal() {
  openModal('joinRoomModal');
}

// 加入视频房间
async function joinVideoRoom(event) {
  event.preventDefault();
  
  const roomId = document.getElementById('joinRoomId').value.trim();
  const password = document.getElementById('joinRoomPassword').value.trim();
  
  if (!roomId) {
    showToast('请输入房间号', 'error');
    return;
  }
  
  // 直接跳转到房间页面，密码验证在房间页面进行
  closeModal('joinRoomModal');
  const url = password ? `room.html?room=${roomId}&pwd=${encodeURIComponent(password)}` : `room.html?room=${roomId}`;
  window.open(url, '_blank');
}

// ==========================================
// WebSocket 实时通信
// ==========================================

// WebSocket 连接地址（ws 或 wss）
const WS_URL = BASE_URL.replace(/^http/, 'ws') + '/ws';

// 连接 WebSocket
async function connectWebSocket() {
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    console.log('📡 WebSocket 已连接');
    return;
  }
  
  if (!state.accessToken) {
    console.log('📡 无 Token，跳过 WebSocket 连接');
    return;
  }
  
  // 确保 Token 有效
  const claims = decodeJwt(state.accessToken);
  if (!claims || claims.exp * 1000 < Date.now()) {
    console.log('📡 Token 已过期，尝试刷新...');
    if (state.refreshToken) {
      const refreshed = await refreshTokenRequest();
      if (!refreshed) {
        console.log('📡 Token 刷新失败，无法连接 WebSocket');
        return;
      }
    } else {
      console.log('📡 无 Refresh Token，无法连接 WebSocket');
      return;
    }
  }
  
  const url = `${WS_URL}?token=${encodeURIComponent(state.accessToken)}`;
  console.log('📡 正在连接 WebSocket...');
  
  try {
    state.ws = new WebSocket(url);
    
    state.ws.onopen = () => {
      console.log('📡 WebSocket 连接成功');
      state.wsConnected = true;
      updateWsStatus(true);
      
      // 清除重连定时器
      if (state.wsReconnectTimer) {
        clearTimeout(state.wsReconnectTimer);
        state.wsReconnectTimer = null;
      }
      
      // 启动心跳定时器（每 25 秒发送一次 ping，后端超时是 60 秒）
      if (state.wsPingInterval) {
        clearInterval(state.wsPingInterval);
      }
      state.wsPingInterval = setInterval(() => {
        wsSendPing();
      }, 25000);
    };
    
    state.ws.onclose = (e) => {
      console.log(`📡 WebSocket 连接关闭: ${e.code} ${e.reason}`);
      state.wsConnected = false;
      state.ws = null;
      updateWsStatus(false);
      
      // 清除心跳定时器
      if (state.wsPingInterval) {
        clearInterval(state.wsPingInterval);
        state.wsPingInterval = null;
      }
      
      // 自动重连（如果有 token）
      if (state.accessToken && !state.wsReconnectTimer) {
        console.log('📡 将在 5 秒后尝试重连...');
        state.wsReconnectTimer = setTimeout(() => {
          state.wsReconnectTimer = null;
          connectWebSocket();
        }, 5000);
      }
    };
    
    state.ws.onerror = (e) => {
      console.error('📡 WebSocket 错误:', e);
    };
    
    state.ws.onmessage = (event) => {
      handleWsMessage(event.data);
    };
    
  } catch (err) {
    console.error('📡 WebSocket 连接失败:', err);
    updateWsStatus(false);
  }
}

// 断开 WebSocket
function disconnectWebSocket() {
  // 清除心跳定时器
  if (state.wsPingInterval) {
    clearInterval(state.wsPingInterval);
    state.wsPingInterval = null;
  }
  
  // 清除重连定时器
  if (state.wsReconnectTimer) {
    clearTimeout(state.wsReconnectTimer);
    state.wsReconnectTimer = null;
  }
  
  if (state.ws) {
    state.ws.close();
    state.ws = null;
  }
  
  state.wsConnected = false;
  updateWsStatus(false);
  console.log('📡 WebSocket 已断开');
}

// 处理 WebSocket 消息
function handleWsMessage(data) {
  try {
    const msg = JSON.parse(data);
    console.log('📨 收到 WebSocket 消息:', msg.type, msg);
    
    switch (msg.type) {
      case 'connected':
        handleWsConnected(msg);
        break;
        
      case 'new_message':
        handleWsNewMessage(msg);
        break;
        
      case 'message_recalled':
        handleWsMessageRecalled(msg);
        break;
        
      case 'read_sync':
        handleWsReadSync(msg);
        break;
        
      case 'system_notification':
        handleWsSystemNotification(msg);
        break;
        
      case 'pong':
        // 心跳响应，忽略
        break;
        
      case 'error':
        console.error('📡 WebSocket 错误:', msg.code, msg.message);
        showToast(`WebSocket: ${msg.message}`, 'error');
        break;
        
      default:
        console.log('📡 未知消息类型:', msg.type);
    }
  } catch (err) {
    console.error('📡 解析消息失败:', err);
  }
}

// 处理连接成功消息
function handleWsConnected(msg) {
  state.unreadSummary = msg.unread_summary;
  
  // 更新未读角标
  const totalUnread = msg.unread_summary.total_count || 0;
  updateUnreadBadge(totalUnread);
  
  // 更新 state.conversations 中的未读数
  if (msg.unread_summary.friend_unreads) {
    msg.unread_summary.friend_unreads.forEach(u => {
      const conv = state.conversations.find(c => c.type === 'friend' && c.id === u.friend_id);
      if (conv) {
        conv.unreadCount = u.unread_count;
        conv.lastMessage = u.last_message_preview;
        conv.time = u.last_message_time;
      }
    });
  }
  
  if (msg.unread_summary.group_unreads) {
    msg.unread_summary.group_unreads.forEach(u => {
      const conv = state.conversations.find(c => c.type === 'group' && c.id === u.group_id);
      if (conv) {
        conv.unreadCount = u.unread_count;
        conv.lastMessage = u.last_message_preview;
        conv.time = u.last_message_time;
      }
    });
  }
  
  // 按未读数和时间排序会话列表
  state.conversations.sort((a, b) => {
    // 有未读的排前面
    if ((b.unreadCount || 0) !== (a.unreadCount || 0)) {
      return (b.unreadCount || 0) - (a.unreadCount || 0);
    }
    // 时间新的排前面
    return new Date(b.time || 0) - new Date(a.time || 0);
  });
  
  // 重新渲染会话列表（带未读角标）
  renderConversations();
  
  showToast('实时消息已连接', 'success');
}

// 处理新消息通知
function handleWsNewMessage(msg) {
  const { source_type, source_id, message_uuid, sender_id, sender_nickname, preview, message_type, timestamp } = msg;
  
  // 如果当前正在查看这个会话，直接加载新消息并标记已读（不增加未读数）
  if (state.currentChat && 
      state.currentChat.type === source_type && 
      state.currentChat.id === source_id) {
    // 记录收到新消息时用户是否在底部附近
    const wasNearBottom = isNearBottom();
    
    // 只更新最后消息预览，不增加未读数
    updateConversationUnread(source_type, source_id, 0, preview, timestamp);
    // 加载消息，根据用户是否在底部决定是否滚动
    loadMessages(wasNearBottom);
    wsSendMarkRead(source_type, source_id);
  } else {
    // 不在当前会话，增加未读数
    updateConversationUnread(source_type, source_id, '+1', preview, timestamp);
    
    // 重新计算总未读数（从 state.conversations 计算，避免重复）
    recalculateTotalUnread();
    
    // 显示通知
    const title = source_type === 'friend' ? sender_nickname : `群消息`;
    showToast(`${title}: ${preview}`, 'info');
    
    // 浏览器通知（如果允许）
    if (Notification.permission === 'granted') {
      new Notification(title, { body: preview, tag: message_uuid });
    }
  }
}

// 处理消息撤回通知
function handleWsMessageRecalled(msg) {
  const { source_type, source_id, message_uuid, recalled_by } = msg;
  
  // 如果当前正在查看这个会话，重新加载消息
  if (state.currentChat && 
      state.currentChat.type === source_type && 
      state.currentChat.id === source_id) {
    loadMessages();
  }
  
  showToast('有消息被撤回', 'info');
}

// 处理已读同步通知
function handleWsReadSync(msg) {
  const { source_type, source_id, reader_id, read_at } = msg;
  
  // 可以在这里更新 UI 显示对方已读状态
  console.log(`📖 ${reader_id} 已读 ${source_type}/${source_id}`);
}

// 处理系统通知
function handleWsSystemNotification(msg) {
  const { notification_type, data } = msg;
  
  switch (notification_type) {
    case 'friend_request':
      showToast(`${data.from_nickname || '用户'} 请求添加好友`, 'info');
      loadPendingRequests();
      break;
      
    case 'friend_request_approved':
      showToast('好友请求已通过', 'success');
      loadFriends();
      break;
      
    case 'friend_request_rejected':
      showToast('好友请求被拒绝', 'warning');
      break;
      
    case 'group_invite':
      showToast(`收到群聊邀请`, 'info');
      break;
      
    case 'group_join_request':
      showToast('有新的入群申请', 'info');
      break;
      
    case 'group_join_approved':
      showToast('入群申请已通过', 'success');
      loadMyGroups();
      break;
      
    case 'group_removed':
      showToast('你已被移出群聊', 'warning');
      loadMyGroups();
      break;
      
    case 'group_disbanded':
      showToast('群聊已解散', 'warning');
      loadMyGroups();
      break;
      
    case 'group_notice_updated':
      showToast('群公告已更新', 'info');
      break;
      
    default:
      console.log('📢 系统通知:', notification_type, data);
  }
}

// 发送标记已读
function wsSendMarkRead(targetType, targetId) {
  if (!state.ws || state.ws.readyState !== WebSocket.OPEN) {
    return;
  }
  
  state.ws.send(JSON.stringify({
    type: 'mark_read',
    target_type: targetType,
    target_id: targetId
  }));
  
  // 更新本地未读数
  updateConversationUnread(targetType, targetId, 0);
  
  // 更新总未读数
  recalculateTotalUnread();
}

// 发送心跳
function wsSendPing() {
  if (!state.ws || state.ws.readyState !== WebSocket.OPEN) {
    return;
  }
  
  state.ws.send(JSON.stringify({ type: 'ping' }));
}

// 更新 WebSocket 连接状态显示
function updateWsStatus(connected) {
  const indicator = document.getElementById('wsStatusIndicator');
  const text = document.getElementById('wsStatusText');
  
  if (indicator) {
    indicator.className = `ws-status-indicator ${connected ? 'connected' : 'disconnected'}`;
  }
  
  if (text) {
    text.textContent = connected ? '已连接' : '未连接';
  }
}

// 更新未读角标
function updateUnreadBadge(count) {
  const badge = document.getElementById('unreadBadge');
  if (badge) {
    if (count > 0) {
      badge.textContent = count > 99 ? '99+' : count;
      badge.style.display = 'flex';
    } else {
      badge.style.display = 'none';
    }
  }
}

// 更新会话的未读数
function updateConversationUnread(type, id, count, lastMessage, lastTime) {
  const chatId = `${type}-${id}`;
  
  // 1. 更新 state.conversations 中的未读数（确保重新渲染时保留状态）
  const convIndex = state.conversations.findIndex(c => c.type === type && c.id === id);
  if (convIndex !== -1) {
    if (count === '+1') {
      const current = state.conversations[convIndex].unreadCount || 0;
      count = current + 1;
    }
    state.conversations[convIndex].unreadCount = count;
    
    if (lastMessage) {
      state.conversations[convIndex].lastMessage = lastMessage;
    }
    if (lastTime) {
      state.conversations[convIndex].time = lastTime;
    }
    
    // 如果有新消息，将该会话移动到顶部
    if (count > 0 || lastMessage) {
      const conv = state.conversations.splice(convIndex, 1)[0];
      state.conversations.unshift(conv);
    }
  }
  
  // 2. 更新 DOM 元素
  const item = document.querySelector(`.conversation-item[data-chat-id="${chatId}"]`);
  
  if (item) {
    const badge = item.querySelector('.unread-badge');
    if (badge) {
      if (count > 0) {
        badge.textContent = count > 99 ? '99+' : count;
        badge.style.display = 'flex';
      } else {
        badge.style.display = 'none';
      }
    }
    
    // 更新最后消息预览
    if (lastMessage) {
      const preview = item.querySelector('.conversation-preview');
      if (preview) {
        preview.textContent = lastMessage;
      }
    }
    
    // 更新时间
    if (lastTime) {
      const time = item.querySelector('.conversation-time');
      if (time) {
        time.textContent = formatTime(lastTime);
      }
    }
    
    // 移动到列表顶部
    const list = item.parentElement;
    if (list && list.firstChild !== item) {
      list.insertBefore(item, list.firstChild);
    }
  }
}

// 重新计算总未读数（从 state.conversations 计算，更可靠）
function recalculateTotalUnread() {
  let total = 0;
  state.conversations.forEach(c => {
    total += (c.unreadCount || 0);
  });
  updateUnreadBadge(total);
}

// 切换 WebSocket 连接（点击状态指示器）
function toggleWsConnection() {
  if (state.wsConnected) {
    disconnectWebSocket();
    showToast('已断开实时消息连接', 'info');
  } else {
    if (!state.accessToken) {
      showToast('请先登录', 'warning');
      return;
    }
    connectWebSocket();
    showToast('正在连接实时消息...', 'info');
  }
}

// ==========================================
// 认证功能
// ==========================================

// 检查登录状态（同步版本，仅检查不刷新）
function checkAuth() {
  if (!state.accessToken) {
    openModal('authModal');
    return false;
  }
  
  const claims = decodeJwt(state.accessToken);
  if (!claims || claims.exp * 1000 < Date.now()) {
    // Token 过期
    return false;
  }
  
  return true;
}

// 确保 Token 有效（异步版本，会自动刷新）
async function ensureAuth() {
  if (!state.accessToken) {
    openModal('authModal');
    return false;
  }
  
  const claims = decodeJwt(state.accessToken);
  if (!claims || claims.exp * 1000 < Date.now()) {
    // Token 过期，尝试刷新
    if (state.refreshToken) {
      const refreshed = await refreshTokenRequest();
      return refreshed;
    } else {
      openModal('authModal');
    return false;
    }
  }
  
  return true;
}

// 切换认证标签
function switchAuthTab(tab) {
  document.querySelectorAll('.auth-tab').forEach(t => t.classList.remove('active'));
  document.querySelector(`.auth-tab[onclick*="${tab}"]`).classList.add('active');
  
  document.getElementById('loginForm').style.display = tab === 'login' ? 'block' : 'none';
  document.getElementById('registerForm').style.display = tab === 'register' ? 'block' : 'none';
}

// 登录
async function handleLogin(e) {
  e.preventDefault();
  
  const user_id = document.getElementById('loginUserId').value.trim();
  const password = document.getElementById('loginPassword').value;
  
  try {
    const result = await api('/api/auth/login', {
      method: 'POST',
      body: { user_id, password, device_info: navigator.userAgent },
      token: null
    });
    
    state.accessToken = result.access_token;
    state.refreshToken = result.refresh_token;
    localStorage.setItem('accessToken', result.access_token);
    localStorage.setItem('refreshToken', result.refresh_token);
    
    const claims = decodeJwt(result.access_token);
    state.currentUser = { user_id: claims.sub };
    localStorage.setItem('currentUser', JSON.stringify(state.currentUser));
    
    closeModal('authModal');
    showToast('登录成功', 'success');
    await initApp();
    
    // 连接 WebSocket
    await connectWebSocket();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 注册
async function handleRegister(e) {
  e.preventDefault();
  
  const user_id = document.getElementById('regUserId').value.trim();
  const nickname = document.getElementById('regNickname').value.trim();
  const password = document.getElementById('regPassword').value;
  const email = document.getElementById('regEmail').value.trim();
  
  try {
    const body = { user_id, nickname, password };
    if (email) body.email = email;
    
    await api('/api/auth/register', { method: 'POST', body, token: null });
    
    showToast('注册成功，请登录', 'success');
    switchAuthTab('login');
    document.getElementById('loginUserId').value = user_id;
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 刷新 Token
async function refreshTokenRequest() {
  try {
    console.log('🔄 正在刷新 Token...');
    const result = await api('/api/auth/refresh', {
      method: 'POST',
      body: { refresh_token: state.refreshToken },
      token: null
    });
    
    state.accessToken = result.access_token;
    localStorage.setItem('accessToken', result.access_token);
    console.log('✅ Token 刷新成功');
    
    // 刷新成功后重新连接 WebSocket
    if (!state.wsConnected) {
      connectWebSocket();
    }
    
    return true;
  } catch (e) {
    console.error('❌ Token 刷新失败:', e);
    // 刷新失败，需要重新登录
    state.accessToken = '';
    state.refreshToken = '';
    localStorage.removeItem('accessToken');
    localStorage.removeItem('refreshToken');
    openModal('authModal');
    return false;
  }
}

// 登出
async function logout() {
  // 断开 WebSocket
  disconnectWebSocket();
  
  try {
    await api('/api/auth/logout', { method: 'POST' });
  } catch {}
  
  state.accessToken = '';
  state.refreshToken = '';
  state.currentUser = null;
  state.unreadSummary = null;
  localStorage.removeItem('accessToken');
  localStorage.removeItem('refreshToken');
  localStorage.removeItem('currentUser');
  
  closeModal('profileModal');
  openModal('authModal');
  showToast('已退出登录');
}

// ==========================================
// 用户资料
// ==========================================

// 显示个人资料
async function showProfileModal() {
  if (!checkAuth()) return;
  
  try {
    const result = await api('/api/profile');
    const profile = result.data;
    
    document.getElementById('profileUserId').value = profile.user_id;
    document.getElementById('profileNickname').value = profile.user_nickname || '';
    document.getElementById('profileEmail').value = profile.user_email || '';
    document.getElementById('profileSignature').value = profile.user_signature || '';
    
    if (profile.user_avatar_url) {
      document.getElementById('profileAvatar').innerHTML = `<img src="${profile.user_avatar_url}" alt="头像">`;
    }
    
    openModal('profileModal');
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 更新资料
async function updateProfile(e) {
  e.preventDefault();
  
  const email = document.getElementById('profileEmail').value.trim();
  const signature = document.getElementById('profileSignature').value.trim();
  
  try {
    const body = {};
    if (email) body.email = email;
    if (signature) body.signature = signature;
    
    await api('/api/profile', { method: 'PUT', body });
    showToast('资料已更新', 'success');
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 修改密码
async function changePassword(e) {
  e.preventDefault();
  
  const old_password = document.getElementById('oldPassword').value;
  const new_password = document.getElementById('newPassword').value;
  
  try {
    await api('/api/profile/password', { method: 'PUT', body: { old_password, new_password } });
    showToast('密码已修改', 'success');
    document.getElementById('oldPassword').value = '';
    document.getElementById('newPassword').value = '';
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 上传头像
async function uploadAvatar(files) {
  if (!files || !files[0]) return;
  
  const file = files[0];
  if (file.size > 10 * 1024 * 1024) {
    showToast('头像大小不能超过 10MB', 'error');
    return;
  }
  
  try {
    const formData = new FormData();
    formData.append('avatar', file);
    
    const result = await api('/api/profile/avatar', { method: 'POST', formData });
    
    if (result.avatar_url || result.data?.avatar_url) {
      const url = result.avatar_url || result.data.avatar_url;
      document.getElementById('profileAvatar').innerHTML = `<img src="${url}" alt="头像">`;
      document.getElementById('currentUserAvatar').innerHTML = `<img src="${url}" alt="头像">`;
      showToast('头像已更新', 'success');
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 设置
function showSettingsModal() {
  openModal('settingsModal');
}

// 显示设备列表
async function showDevices() {
  try {
    const result = await api('/api/auth/devices');
    const container = document.getElementById('devicesList');
    
    if (result.devices && result.devices.length > 0) {
      container.innerHTML = result.devices.map(d => `
        <div class="device-item">
          <div class="device-info">
            <div class="device-name">
              ${d.device_info || '未知设备'}
              ${d.is_current ? '<span class="device-badge">当前</span>' : ''}
            </div>
            <div class="device-meta">最后活跃: ${formatTime(d.last_used_at)}</div>
          </div>
          ${!d.is_current ? `<button class="btn-secondary btn-sm" onclick="revokeDevice('${d.device_id}')">移除</button>` : ''}
        </div>
      `).join('');
    } else {
      container.innerHTML = '<p class="text-muted">无设备</p>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 移除设备
async function revokeDevice(deviceId) {
  try {
    await api(`/api/auth/devices/${deviceId}`, { method: 'DELETE' });
    showToast('设备已移除', 'success');
    showDevices();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 面板切换
// ==========================================

function switchPanel(panel) {
  // 更新导航按钮状态
  document.querySelectorAll('.nav-btn[data-panel]').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panel);
  });
  
  // 切换面板显示
  document.querySelectorAll('.panel-content').forEach(p => p.style.display = 'none');
  document.getElementById(`panel-${panel}`).style.display = 'block';
  
  // 加载对应数据
  switch (panel) {
    case 'chats':
      loadConversations();
      break;
    case 'contacts':
      loadFriends();
      loadPendingRequests();
      break;
    case 'groups':
      loadMyGroups();
      loadGroupInvitations();
      break;
    case 'files':
      loadMyFiles();
      break;
  }
}

function switchContactTab(tab) {
  document.querySelectorAll('#panel-contacts .panel-tab').forEach(t => t.classList.remove('active'));
  event.target.classList.add('active');
  
  document.getElementById('friendsList').style.display = tab === 'friends' ? 'block' : 'none';
  document.getElementById('pendingList').style.display = tab === 'pending' ? 'block' : 'none';
  document.getElementById('sentList').style.display = tab === 'sent' ? 'block' : 'none';
}

function switchGroupTab(tab) {
  document.querySelectorAll('#panel-groups .panel-tab').forEach(t => t.classList.remove('active'));
  event.target.classList.add('active');
  
  document.getElementById('myGroupsList').style.display = tab === 'myGroups' ? 'block' : 'none';
  document.getElementById('groupInvitationsList').style.display = tab === 'invitations' ? 'block' : 'none';
}

function switchFileTab(tab) {
  document.querySelectorAll('#panel-files .panel-tab').forEach(t => t.classList.remove('active'));
  event.target.classList.add('active');
  
  document.getElementById('myFilesList').style.display = tab === 'myFiles' ? 'block' : 'none';
  document.getElementById('uploadPanel').style.display = tab === 'upload' ? 'block' : 'none';
}

// ==========================================
// 好友功能
// ==========================================

// 加载好友列表
async function loadFriends() {
  try {
    const result = await api('/api/friends');
    state.friends = result.items || [];
    renderFriendsList();
  } catch (err) {
    console.error('加载好友失败:', err);
  }
}

function renderFriendsList() {
  const container = document.getElementById('friendsList');
  
  if (state.friends.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">👥</div>
        <p>暂无好友</p>
        <button class="btn-primary" onclick="showAddFriendModal()">添加好友</button>
      </div>
    `;
    return;
  }
  
  container.innerHTML = state.friends.map(f => {
    const avatarHtml = f.friend_avatar_url 
      ? `<img src="${f.friend_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`
      : '👤';
    return `
      <div class="contact-item" onclick="openChat('friend', '${f.friend_id}', '${f.friend_nickname || f.friend_id}')">
        <div class="item-avatar">${avatarHtml}</div>
        <div class="item-info">
          <div class="item-name">${f.friend_nickname || f.friend_id}</div>
          <div class="item-preview">${f.friend_id}</div>
        </div>
      </div>
    `;
  }).join('');
}

// 加载待处理请求
async function loadPendingRequests() {
  try {
    const result = await api('/api/friends/requests/pending');
    renderPendingList(result.items || []);
  } catch (err) {
    console.error('加载待处理请求失败:', err);
  }
}

function renderPendingList(items) {
  const container = document.getElementById('pendingList');
  
  if (items.length === 0) {
    container.innerHTML = '<div class="empty-state"><p>无待处理请求</p></div>';
    return;
  }
  
  container.innerHTML = items.map(item => `
    <div class="contact-item">
      <div class="item-avatar">👤</div>
      <div class="item-info">
        <div class="item-name">${item.request_user_id}</div>
        <div class="item-preview">${item.request_message || '请求添加你为好友'}</div>
      </div>
      <div class="item-actions">
        <button class="btn-accept" onclick="approveFriendRequest('${item.request_user_id}')">同意</button>
        <button class="btn-reject" onclick="rejectFriendRequest('${item.request_user_id}')">拒绝</button>
      </div>
    </div>
  `).join('');
}

// 加载已发送请求
async function loadSentRequests() {
  try {
    const result = await api('/api/friends/requests/sent');
    renderSentList(result.items || []);
  } catch (err) {
    console.error('加载已发送请求失败:', err);
  }
}

function renderSentList(items) {
  const container = document.getElementById('sentList');
  
  if (items.length === 0) {
    container.innerHTML = '<div class="empty-state"><p>无已发送请求</p></div>';
    return;
  }
  
  container.innerHTML = items.map(item => `
    <div class="contact-item">
      <div class="item-avatar">👤</div>
      <div class="item-info">
        <div class="item-name">${item.sent_to_user_id}</div>
        <div class="item-preview">${formatTime(item.sent_time)}</div>
      </div>
      <span class="item-badge" style="background: var(--warning);">等待中</span>
    </div>
  `).join('');
}

// 显示添加好友弹窗
function showAddFriendModal() {
  closeAddMenu();
  openModal('addFriendModal');
}

// 发送好友请求
async function submitFriendRequest(e) {
  e.preventDefault();
  
  const target_user_id = document.getElementById('addFriendId').value.trim();
  const reason = document.getElementById('addFriendMsg').value.trim();
  
  if (!target_user_id) {
    showToast('请输入用户ID', 'error');
    return;
  }
  
  try {
    const claims = decodeJwt(state.accessToken);
    const body = {
      user_id: claims.sub,
      target_user_id,
      request_time: new Date().toISOString()
    };
    if (reason) body.reason = reason;
    
    await api('/api/friends/requests', { method: 'POST', body });
    
    showToast('好友请求已发送', 'success');
    closeModal('addFriendModal');
    document.getElementById('addFriendId').value = '';
    document.getElementById('addFriendMsg').value = '';
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 同意好友请求
async function approveFriendRequest(applicantId) {
  try {
    const claims = decodeJwt(state.accessToken);
    await api('/api/friends/requests/approve', {
      method: 'POST',
      body: {
        user_id: claims.sub,
        applicant_user_id: applicantId,
        approved_time: new Date().toISOString()
      }
    });
    
    showToast('已添加好友', 'success');
    loadPendingRequests();
    loadFriends();
    loadConversations();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 拒绝好友请求
async function rejectFriendRequest(applicantId) {
  try {
    const claims = decodeJwt(state.accessToken);
    await api('/api/friends/requests/reject', {
      method: 'POST',
      body: {
        user_id: claims.sub,
        applicant_user_id: applicantId
      }
    });
    
    showToast('已拒绝', 'success');
    loadPendingRequests();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 删除好友
async function removeFriend(friendId) {
  if (!confirm(`确定要删除好友 ${friendId} 吗？`)) return;
  
  try {
    const claims = decodeJwt(state.accessToken);
    await api('/api/friends/remove', {
      method: 'POST',
      body: {
        user_id: claims.sub,
        friend_user_id: friendId,
        remove_time: new Date().toISOString()
      }
    });
    
    showToast('已删除好友', 'success');
    loadFriends();
    
    if (state.currentChat?.type === 'friend' && state.currentChat.id === friendId) {
      closeChat();
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 群聊功能
// ==========================================

// 加载我的群聊
async function loadMyGroups() {
  try {
    const result = await api('/api/groups/my');
    state.groups = result.data || [];
    renderGroupsList();
  } catch (err) {
    console.error('加载群聊失败:', err);
  }
}

function renderGroupsList() {
  const container = document.getElementById('myGroupsList');
  
  if (state.groups.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">👥</div>
        <p>暂无群聊</p>
        <button class="btn-primary" onclick="showCreateGroupModal()">创建群聊</button>
      </div>
    `;
    return;
  }
  
  container.innerHTML = state.groups.map(g => `
    <div class="group-item" onclick="openChat('group', '${g.group_id}', '${g.group_name}')">
      <div class="item-avatar">👥</div>
      <div class="item-info">
        <div class="item-name">${g.group_name}</div>
        <div class="item-preview">${g.member_count || 0}人</div>
      </div>
    </div>
  `).join('');
}

// 加载群邀请
async function loadGroupInvitations() {
  try {
    const result = await api('/api/groups/invitations');
    renderGroupInvitations(result.data?.invitations || []);
  } catch (err) {
    console.error('加载群邀请失败:', err);
  }
}

function renderGroupInvitations(items) {
  const container = document.getElementById('groupInvitationsList');
  
  if (items.length === 0) {
    container.innerHTML = '<div class="empty-state"><p>无群邀请</p></div>';
    return;
  }
  
  container.innerHTML = items.map(item => `
    <div class="group-item">
      <div class="item-avatar">👥</div>
      <div class="item-info">
        <div class="item-name">${item.group_name || '群聊'}</div>
        <div class="item-preview">${item.inviter_id} 邀请你加入</div>
      </div>
      <div class="item-actions">
        <button class="btn-accept" onclick="acceptGroupInvitation('${item.request_id}')">接受</button>
        <button class="btn-reject" onclick="rejectGroupInvitation('${item.request_id}')">拒绝</button>
      </div>
    </div>
  `).join('');
}

// 显示创建群聊弹窗
function showCreateGroupModal() {
  closeAddMenu();
  openModal('createGroupModal');
}

// 创建群聊
async function createGroup(e) {
  e.preventDefault();
  
  const group_name = document.getElementById('groupName').value.trim();
  const group_description = document.getElementById('groupDesc').value.trim();
  const join_mode = document.getElementById('groupJoinMode').value;
  
  try {
    const body = { group_name, join_mode };
    if (group_description) body.group_description = group_description;
    
    const result = await api('/api/groups', { method: 'POST', body });
    
    showToast('群聊创建成功', 'success');
    closeModal('createGroupModal');
    
    document.getElementById('groupName').value = '';
    document.getElementById('groupDesc').value = '';
    
    loadMyGroups();
    
    // 打开新创建的群聊
    if (result.data?.group_id) {
      openChat('group', result.data.group_id, group_name);
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 接受群邀请
async function acceptGroupInvitation(requestId) {
  try {
    await api(`/api/groups/invitations/${requestId}/accept`, { method: 'POST' });
    showToast('已加入群聊', 'success');
    loadGroupInvitations();
    loadMyGroups();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 拒绝群邀请
async function rejectGroupInvitation(requestId) {
  try {
    await api(`/api/groups/invitations/${requestId}/reject`, { method: 'POST' });
    showToast('已拒绝', 'success');
    loadGroupInvitations();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 邀请成员
async function showInviteMemberModal() {
  if (!state.currentChat || state.currentChat.type !== 'group') return;
  
  // 加载好友列表用于选择
  const container = document.getElementById('inviteFriendsList');
  container.innerHTML = state.friends.map(f => `
    <label class="checkbox-item">
      <input type="checkbox" value="${f.friend_id}">
      <span>${f.friend_nickname || f.friend_id}</span>
    </label>
  `).join('');
  
  openModal('inviteMemberModal');
}

async function inviteToGroup(e) {
  e.preventDefault();
  
  if (!state.currentChat || state.currentChat.type !== 'group') return;
  
  const checkboxes = document.querySelectorAll('#inviteFriendsList input:checked');
  const user_ids = Array.from(checkboxes).map(cb => cb.value);
  const message = document.getElementById('inviteMessage').value.trim();
  
  if (user_ids.length === 0) {
    showToast('请选择要邀请的好友', 'error');
    return;
  }
  
  try {
    const body = { user_ids };
    if (message) body.message = message;
    
    await api(`/api/groups/${state.currentChat.id}/invite`, { method: 'POST', body });
    
    showToast('邀请已发送', 'success');
    closeModal('inviteMemberModal');
    loadGroupMembers(state.currentChat.id);
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 加载群成员
async function loadGroupMembers(groupId) {
  try {
    const result = await api(`/api/groups/${groupId}/members`);
    state.groupMembers[groupId] = result.data?.members || [];
    return state.groupMembers[groupId];
  } catch (err) {
    console.error('加载群成员失败:', err);
    return [];
  }
}

// 群公告
async function showGroupNotices() {
  if (!state.currentChat || state.currentChat.type !== 'group') return;
  
  try {
    const result = await api(`/api/groups/${state.currentChat.id}/notices`);
    const notices = result.data?.notices || [];
    
    const container = document.getElementById('noticeList');
    
    if (notices.length === 0) {
      container.innerHTML = '<div class="empty-state"><p>暂无公告</p></div>';
    } else {
      container.innerHTML = notices.map(n => `
        <div class="notice-item ${n.is_pinned ? 'pinned' : ''}">
          <div class="notice-title">
            ${n.title}
            ${n.is_pinned ? '<span class="notice-pin-badge">置顶</span>' : ''}
          </div>
          <div class="notice-content">${n.content}</div>
          <div class="notice-meta">${n.publisher_id} · ${formatTime(n.created_at)}</div>
        </div>
      `).join('');
    }
    
    // 检查权限，显示发布按钮
    const members = state.groupMembers[state.currentChat.id] || [];
    const claims = decodeJwt(state.accessToken);
    const myMember = members.find(m => m.user_id === claims?.sub);
    
    document.getElementById('noticeActions').style.display = 
      (myMember?.role === 'owner' || myMember?.role === 'admin') ? 'flex' : 'none';
    
    document.getElementById('publishNoticeForm').style.display = 'none';
    
    openModal('groupNoticeModal');
  } catch (err) {
    showToast(err.message, 'error');
  }
}

function showPublishNoticeForm() {
  document.getElementById('publishNoticeForm').style.display = 'block';
  document.getElementById('noticeActions').style.display = 'none';
}

function hidePublishNoticeForm() {
  document.getElementById('publishNoticeForm').style.display = 'none';
  document.getElementById('noticeActions').style.display = 'flex';
}

async function publishNotice(e) {
  e.preventDefault();
  
  const title = document.getElementById('noticeTitle').value.trim();
  const content = document.getElementById('noticeContent').value.trim();
  const is_pinned = document.getElementById('noticePinned').checked;
  
  try {
    await api(`/api/groups/${state.currentChat.id}/notices`, {
      method: 'POST',
      body: { title, content, is_pinned }
    });
    
    showToast('公告已发布', 'success');
    document.getElementById('noticeTitle').value = '';
    document.getElementById('noticeContent').value = '';
    document.getElementById('noticePinned').checked = false;
    
    showGroupNotices();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 邀请码
function showInviteCodeModal() {
  document.getElementById('generatedCodeResult').style.display = 'none';
  openModal('inviteCodeModal');
}

async function generateInviteCode(e) {
  e.preventDefault();
  
  const max_uses = parseInt(document.getElementById('inviteCodeMaxUses').value);
  const expires_in_hours = parseInt(document.getElementById('inviteCodeExpires').value);
  
  try {
    const result = await api(`/api/groups/${state.currentChat.id}/invite_codes`, {
      method: 'POST',
      body: { max_uses, expires_in_hours }
    });
    
    document.getElementById('inviteCodeValue').textContent = result.data?.code || '';
    document.getElementById('generatedCodeResult').style.display = 'block';
    showToast('邀请码已生成', 'success');
  } catch (err) {
    showToast(err.message, 'error');
  }
}

function copyInviteCode() {
  const code = document.getElementById('inviteCodeValue').textContent;
  navigator.clipboard.writeText(code).then(() => {
    showToast('已复制到剪贴板', 'success');
  });
}

function showUseInviteCodeModal() {
  closeAddMenu();
  openModal('useInviteCodeModal');
}

async function joinByInviteCode(e) {
  e.preventDefault();
  
  const code = document.getElementById('joinInviteCode').value.trim();
  
  try {
    await api('/api/groups/join_by_code', { method: 'POST', body: { code } });
    showToast('已加入群聊', 'success');
    closeModal('useInviteCodeModal');
    document.getElementById('joinInviteCode').value = '';
    loadMyGroups();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 会话和聊天
// ==========================================

// 加载会话列表
async function loadConversations() {
  // 保存旧的未读计数（避免刷新时丢失）
  const oldUnreads = {};
  state.conversations.forEach(c => {
    if (c.unreadCount > 0) {
      oldUnreads[`${c.type}-${c.id}`] = {
        unreadCount: c.unreadCount,
        lastMessage: c.lastMessage,
        time: c.time
      };
    }
  });
  
  // 合并好友和群聊作为会话
  const conversations = [];
  
  // 好友会话
  for (const f of state.friends) {
    const key = `friend-${f.friend_id}`;
    const old = oldUnreads[key] || {};
    conversations.push({
      type: 'friend',
      id: f.friend_id,
      name: f.friend_nickname || f.friend_id,
      avatarUrl: f.friend_avatar_url,
      lastMessage: old.lastMessage || '',
      time: old.time || f.add_time,
      unreadCount: old.unreadCount || 0
    });
  }
  
  // 群聊会话
  for (const g of state.groups) {
    const key = `group-${g.group_id}`;
    const old = oldUnreads[key] || {};
    conversations.push({
      type: 'group',
      id: g.group_id,
      name: g.group_name,
      avatarUrl: g.group_avatar_url,
      lastMessage: old.lastMessage || '',
      time: old.time || g.created_at,
      memberCount: g.member_count,
      unreadCount: old.unreadCount || 0
    });
  }
  
  // 按未读数和时间排序
  conversations.sort((a, b) => {
    if ((b.unreadCount || 0) !== (a.unreadCount || 0)) {
      return (b.unreadCount || 0) - (a.unreadCount || 0);
    }
    return new Date(b.time || 0) - new Date(a.time || 0);
  });
  
  state.conversations = conversations;
  renderConversations();
}

function renderConversations() {
  const container = document.getElementById('conversationList');
  
  if (state.conversations.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">💬</div>
        <p>暂无会话</p>
        <p class="empty-hint">添加好友或创建群聊开始聊天</p>
      </div>
    `;
    return;
  }
  
  container.innerHTML = state.conversations.map(c => {
    const chatId = `${c.type}-${c.id}`;
    const isActive = state.currentChat?.type === c.type && state.currentChat?.id === c.id;
    const defaultIcon = c.type === 'group' ? '👥' : '👤';
    const avatarHtml = c.avatarUrl 
      ? `<img src="${c.avatarUrl}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='${defaultIcon}'">`
      : defaultIcon;
    
    // 获取该会话的未读数（从缓存或状态中获取）
    const unreadCount = c.unreadCount || 0;
    const unreadBadgeHtml = unreadCount > 0 
      ? `<div class="unread-badge" style="display: flex;">${unreadCount > 99 ? '99+' : unreadCount}</div>`
      : `<div class="unread-badge" style="display: none;">0</div>`;
    
    return `
      <div class="conversation-item ${isActive ? 'active' : ''}" 
           data-chat-id="${chatId}"
           onclick="openChat('${c.type}', '${c.id}', '${c.name}')">
        <div class="item-avatar">${avatarHtml}</div>
        <div class="item-info">
          <div class="item-name">${c.name}</div>
          <div class="item-preview conversation-preview">${c.lastMessage || (c.type === 'group' ? `${c.memberCount || 0}人` : '')}</div>
        </div>
        <div class="item-meta">
          <div class="item-time conversation-time">${formatTime(c.time)}</div>
          ${unreadBadgeHtml}
        </div>
      </div>
    `;
  }).join('');
}

// 打开聊天
async function openChat(type, id, name) {
  state.currentChat = { type, id, name };
  
  // 显示聊天界面
  document.getElementById('welcomeScreen').style.display = 'none';
  document.getElementById('chatView').style.display = 'flex';
  
  // 更新头部
  document.getElementById('chatName').textContent = name;
  
  // 设置头像（从会话列表中获取头像URL）
  const conversation = state.conversations.find(c => c.type === type && c.id === id);
  const avatarUrl = conversation?.avatarUrl;
  if (avatarUrl) {
    document.getElementById('chatAvatar').innerHTML = `<img src="${avatarUrl}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='${type === 'group' ? '👥' : '👤'}'">`;
  } else {
    document.getElementById('chatAvatar').innerHTML = type === 'group' ? '👥' : '👤';
  }
  
  // 群聊特有按钮
  document.getElementById('btnGroupNotice').style.display = type === 'group' ? 'inline-flex' : 'none';
  
  // 群聊显示成员数
  if (type === 'group') {
    const members = await loadGroupMembers(id);
    document.getElementById('chatStatus').textContent = `${members.length}人`;
  } else {
    document.getElementById('chatStatus').textContent = '';
  }
  
  // 加载消息
  await loadMessages();
  
  // 标记消息已读（清除未读计数）
  wsSendMarkRead(type, id);
  
  // 更新会话列表高亮
  renderConversations();
}

// 关闭聊天
function closeChat() {
  state.currentChat = null;
  document.getElementById('welcomeScreen').style.display = 'flex';
  document.getElementById('chatView').style.display = 'none';
  document.getElementById('infoPanel').style.display = 'none';
  renderConversations();
}

// 加载消息（使用时间戳分页优化）
// scrollToEnd: 是否滚动到底部，默认 true（首次加载），收到新消息时根据用户位置决定
async function loadMessages(scrollToEnd = true) {
  if (!state.currentChat) return;
  
  const { type, id } = state.currentChat;
  const chatKey = `${type}-${id}`;
  const container = document.getElementById('messageList');
  
  try {
    let result;
    
    if (type === 'friend') {
      result = await api(`/api/messages?friend_id=${encodeURIComponent(id)}&limit=50`);
      state.messages[chatKey] = result.messages || [];
      state.hasMore = state.hasMore || {};
      state.hasMore[chatKey] = result.has_more || false;
    } else {
      result = await api(`/api/group-messages?group_id=${encodeURIComponent(id)}&limit=50`);
      state.messages[chatKey] = result.data?.messages || [];
      state.hasMore = state.hasMore || {};
      state.hasMore[chatKey] = result.data?.has_more || false;
    }
    
    renderMessages(state.messages[chatKey], { scrollToEnd });
    
    // 初始化滚动监听（用于自动加载更多历史消息）
    initMessageContainerScrollListener();
  } catch (err) {
    console.error('加载消息失败:', err);
    container.innerHTML = '<div class="empty-state"><p>加载消息失败</p></div>';
  }
}

// 加载更多历史消息（使用时间戳分页）
async function loadMoreMessages() {
  if (!state.currentChat) return;
  
  const { type, id } = state.currentChat;
  const chatKey = `${type}-${id}`;
  const messages = state.messages[chatKey] || [];
  
  // 检查是否还有更多消息
  if (!state.hasMore?.[chatKey] || messages.length === 0) {
    return;
  }
  
  // 显示加载提示
  const hintEl = document.querySelector('.load-more-hint');
  if (hintEl) {
    hintEl.textContent = '⏳ 正在加载...';
  }
  
  // 获取最老消息的时间戳（ISO 8601 格式）
  const oldestMessage = messages[messages.length - 1];
  const beforeTime = oldestMessage?.send_time;
  
  if (!beforeTime) return;
  
  try {
    let result;
    
    if (type === 'friend') {
      result = await api(`/api/messages?friend_id=${encodeURIComponent(id)}&before_time=${encodeURIComponent(beforeTime)}&limit=50`);
      const moreMessages = result.messages || [];
      state.messages[chatKey] = [...messages, ...moreMessages];
      state.hasMore[chatKey] = result.has_more || false;
    } else {
      result = await api(`/api/group-messages?group_id=${encodeURIComponent(id)}&before_time=${encodeURIComponent(beforeTime)}&limit=50`);
      const moreMessages = result.data?.messages || [];
      state.messages[chatKey] = [...messages, ...moreMessages];
      state.hasMore[chatKey] = result.data?.has_more || false;
    }
    
    // 保存当前滚动位置
    const msgContainer = document.getElementById('messageContainer');
    const scrollHeightBefore = msgContainer.scrollHeight;
    const scrollTopBefore = msgContainer.scrollTop;
    
    // 重新渲染消息（不自动滚动到底部）
    renderMessages(state.messages[chatKey], { scrollToEnd: false });
    
    // 恢复滚动位置（加载的历史消息在顶部，保持用户看到的内容不变）
    const scrollHeightAfter = msgContainer.scrollHeight;
    msgContainer.scrollTop = scrollHeightAfter - scrollHeightBefore + scrollTopBefore;
    
    if (!state.hasMore[chatKey]) {
      showToast('已加载全部历史消息', 'info');
    }
  } catch (err) {
    console.error('加载更多消息失败:', err);
    showToast('加载更多消息失败', 'error');
  }
}

function renderMessages(messages, options = {}) {
  const { scrollToEnd = true } = options;  // 默认滚动到底部，加载更多时传 false
  const container = document.getElementById('messageList');
  const claims = decodeJwt(state.accessToken);
  const myId = claims?.sub;
  
  if (messages.length === 0) {
    container.innerHTML = '<div class="empty-state"><div class="empty-icon">💬</div><p>暂无消息</p></div>';
    return;
  }
  
  // 翻转消息顺序：后端返回最新在前，前端需要最新在后（底部）
  const sortedMessages = [...messages].reverse();
  
  // 顶部加载提示（滚动到顶部自动加载更多，无需手动点击）
  const chatKey = state.currentChat ? `${state.currentChat.type}-${state.currentChat.id}` : '';
  const hasMore = state.hasMore?.[chatKey];
  const loadMoreHint = hasMore 
    ? `<div class="load-more-container"><span class="load-more-hint">⬆️ 滚动到顶部加载更多历史消息</span></div>` 
    : '';
  
  container.innerHTML = loadMoreHint + sortedMessages.map(msg => {
    const isSelf = msg.sender_id === myId;
    const hasFile = msg.file_uuid && msg.file_uuid !== 'null';
    
    // 获取头像
    let avatarHtml = '👤';
    if (isSelf) {
      // 自己的头像（从 state.currentUser 获取）
      if (state.currentUser?.user_avatar_url) {
        avatarHtml = `<img src="${state.currentUser.user_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`;
      }
    } else if (state.currentChat?.type === 'friend') {
      // 好友头像：从好友列表查找
      const friend = state.friends.find(f => f.friend_id === msg.sender_id);
      if (friend?.friend_avatar_url) {
        avatarHtml = `<img src="${friend.friend_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`;
      }
    } else if (state.currentChat?.type === 'group') {
      // 群成员头像：从群成员列表查找
      const members = state.groupMembers[state.currentChat.id] || [];
      const member = members.find(m => m.user_id === msg.sender_id);
      if (member?.user_avatar_url) {
        avatarHtml = `<img src="${member.user_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`;
      }
    }
    
    let contentHtml = `<div class="message-text">${escapeHtml(msg.message_content)}</div>`;
    
    // 文件消息
    if (hasFile) {
      if (msg.message_type === 'image') {
        // 不设置 src，避免触发无效请求，后续通过 JS 加载
        contentHtml += `<img class="message-image" alt="加载中..." onclick="previewFile('${msg.file_uuid}')" data-uuid="${msg.file_uuid}" data-loaded="false">`;
      } else if (msg.message_type === 'video') {
        contentHtml += `<video class="message-video" controls data-uuid="${msg.file_uuid}" data-loaded="false" preload="none"></video>`;
      } else {
        contentHtml += `
          <div class="message-file" onclick="downloadFile('${msg.file_uuid}')">
            <span class="message-file-icon">📁</span>
            <div class="message-file-info">
              <div class="message-file-name">${msg.message_content}</div>
            </div>
          </div>
        `;
      }
    }
    
    // 获取发送者显示名称（群聊优先使用 sender_nickname，fallback 到 sender_id）
    let senderDisplayName = msg.sender_id;
    if (state.currentChat?.type === 'group') {
      // 优先使用后端 JOIN 返回的 sender_nickname
      if (msg.sender_nickname) {
        senderDisplayName = msg.sender_nickname;
      } else {
        // fallback: 从群成员列表查找群内昵称或用户昵称
        const members = state.groupMembers[state.currentChat.id] || [];
        const member = members.find(m => m.user_id === msg.sender_id);
        if (member?.group_nickname) {
          senderDisplayName = member.group_nickname;
        } else if (member?.user_nickname) {
          senderDisplayName = member.user_nickname;
        }
      }
    }
    
    return `
      <div class="message ${isSelf ? 'self' : ''}">
        <div class="message-avatar">${avatarHtml}</div>
        <div class="message-body">
          ${!isSelf && state.currentChat?.type === 'group' ? `<div class="message-sender">${escapeHtml(senderDisplayName)}</div>` : ''}
          <div class="message-bubble">${contentHtml}</div>
          <div class="message-time">${formatTime(msg.send_time)}</div>
        </div>
      </div>
    `;
  }).join('');
  
  // 只有首次加载时滚动到底部，加载更多历史消息时不滚动
  if (scrollToEnd) {
    scrollToBottom();
  }
  
  // 加载文件预览
  loadFilePreviewsInMessages();
}

// 滚动消息列表到底部（多次尝试确保滚动成功）
function scrollToBottom() {
  const msgContainer = document.getElementById('messageContainer');
  if (!msgContainer) return;
  
  // 立即滚动一次
  msgContainer.scrollTop = msgContainer.scrollHeight;
  
  // requestAnimationFrame 后再滚动一次
  requestAnimationFrame(() => {
    msgContainer.scrollTop = msgContainer.scrollHeight;
  });
  
  // 延迟 50ms 后再滚动一次（确保 DOM 完全渲染）
  setTimeout(() => {
    msgContainer.scrollTop = msgContainer.scrollHeight;
  }, 50);
  
  // 延迟 200ms 后再滚动一次（确保图片等资源加载后）
  setTimeout(() => {
    msgContainer.scrollTop = msgContainer.scrollHeight;
  }, 200);
}

// 检查用户是否在消息列表底部附近（100px 阈值）
function isNearBottom() {
  const msgContainer = document.getElementById('messageContainer');
  if (!msgContainer) return true;
  
  const threshold = 100; // 距离底部 100px 以内算"在底部"
  return (msgContainer.scrollHeight - msgContainer.scrollTop - msgContainer.clientHeight) < threshold;
}

// 检查用户是否在消息列表顶部附近（触发加载更多）
function isNearTop() {
  const msgContainer = document.getElementById('messageContainer');
  if (!msgContainer) return false;
  
  const threshold = 50; // 距离顶部 50px 以内触发加载
  return msgContainer.scrollTop < threshold;
}

// 消息容器滚动事件处理（滚动到顶部自动加载更多）
let isLoadingMore = false;
function handleMessageContainerScroll() {
  if (isLoadingMore) return;
  if (!state.currentChat) return;
  
  const chatKey = `${state.currentChat.type}-${state.currentChat.id}`;
  
  // 检查是否接近顶部且还有更多消息
  if (isNearTop() && state.hasMore?.[chatKey]) {
    isLoadingMore = true;
    loadMoreMessages().finally(() => {
      isLoadingMore = false;
    });
  }
}

// 初始化消息容器滚动监听
function initMessageContainerScrollListener() {
  const msgContainer = document.getElementById('messageContainer');
  if (msgContainer) {
    msgContainer.removeEventListener('scroll', handleMessageContainerScroll);
    msgContainer.addEventListener('scroll', handleMessageContainerScroll);
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// 加载消息中的文件预览
async function loadFilePreviewsInMessages() {
  // 确保 Token 有效（异步等待刷新）
  if (!await ensureAuth()) return;
  
  // 获取所有未加载的图片和视频
  const images = document.querySelectorAll('.message-image[data-uuid][data-loaded="false"]');
  const videos = document.querySelectorAll('.message-video[data-uuid][data-loaded="false"]');
  
  console.log(`📷 加载文件预览: ${images.length} 张图片, ${videos.length} 个视频`);
  
  // 并行加载所有图片（带重试）
  const imagePromises = Array.from(images).map(async (img) => {
    const uuid = img.dataset.uuid;
    if (!uuid) return;
    
    for (let retry = 0; retry < 3; retry++) {
    try {
      const url = await getFilePresignedUrl(uuid);
        if (url) {
          img.src = url;
          img.dataset.loaded = 'true';
          img.alt = '图片';
          console.log(`✅ 图片加载成功: ${uuid}`);
          return;
        }
      } catch (e) {
        console.warn(`图片加载重试 ${retry + 1}/3 (${uuid}):`, e.message);
        if (retry === 2) {
          img.alt = '图片加载失败，点击重试';
          img.onclick = () => retryLoadFile(img, uuid, 'image');
        }
        await new Promise(r => setTimeout(r, 1000 * (retry + 1))); // 退避重试
      }
    }
  });
  
  // 并行加载所有视频（带重试）
  const videoPromises = Array.from(videos).map(async (video) => {
    const uuid = video.dataset.uuid;
    if (!uuid) return;
    
    for (let retry = 0; retry < 3; retry++) {
    try {
      const url = await getFilePresignedUrl(uuid);
        if (url) {
          video.src = url;
          video.dataset.loaded = 'true';
          console.log(`✅ 视频加载成功: ${uuid}`);
          return;
        }
      } catch (e) {
        console.warn(`视频加载重试 ${retry + 1}/3 (${uuid}):`, e.message);
        if (retry === 2) {
          // 视频加载失败时显示提示
          const parent = video.parentElement;
          if (parent) {
            const errorDiv = document.createElement('div');
            errorDiv.className = 'video-error';
            errorDiv.textContent = '视频加载失败，点击重试';
            errorDiv.onclick = () => {
              errorDiv.remove();
              retryLoadFile(video, uuid, 'video');
            };
            parent.appendChild(errorDiv);
          }
        }
        await new Promise(r => setTimeout(r, 1000 * (retry + 1)));
      }
    }
  });
  
  // 等待所有加载完成
  await Promise.allSettled([...imagePromises, ...videoPromises]);
}

// 重试加载文件
async function retryLoadFile(element, uuid, type) {
  try {
    const url = await getFilePresignedUrl(uuid);
    if (url) {
      element.src = url;
      element.dataset.loaded = 'true';
      if (type === 'image') {
        element.alt = '图片';
        element.onclick = () => previewFile(uuid);
      }
      showToast('文件加载成功', 'success');
    }
  } catch (e) {
    showToast('文件加载失败: ' + e.message, 'error');
  }
}

async function getFilePresignedUrl(uuid) {
  // 确保 Token 有效
  if (!await ensureAuth()) {
    throw new Error('认证失败');
  }
  
  // 优先尝试通用文件 API（后端会自动判断 bucket 类型）
  // 通用 API 支持：个人文件、群文件、好友文件
  try {
    const result = await api(`/api/storage/file/${uuid}/presigned_url`, { 
      method: 'POST', 
      body: { operation: 'download' } 
    });
    return result.presigned_url;
  } catch (e) {
    // 如果通用 API 失败（可能是权限问题），尝试好友专用 API
    // 好友专用 API 会验证好友关系
    if (state.currentChat?.type === 'friend') {
      console.log(`通用 API 失败，尝试好友文件 API: ${uuid}`);
      const result = await api(`/api/storage/friends_file/${uuid}/presigned_url`, { 
        method: 'POST', 
        body: { operation: 'download' } 
      });
  return result.presigned_url;
    }
    throw e;
  }
}

// 发送消息
async function sendMessage() {
  if (!state.currentChat) return;
  
  const input = document.getElementById('messageInput');
  const content = input.value.trim();
  
  if (!content) return;
  
  const { type, id } = state.currentChat;
  
  try {
    if (type === 'friend') {
      await api('/api/messages', {
        method: 'POST',
        body: {
          receiver_id: id,
          message_content: content,
          message_type: 'text'
        }
      });
    } else {
      await api('/api/group-messages', {
        method: 'POST',
        body: {
          group_id: id,
          message_content: content,
          message_type: 'text'
        }
      });
    }
    
    input.value = '';
    input.style.height = 'auto';
    loadMessages();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 发送文件消息（预签名直传MinIO，真实进度条）
async function sendFileMessage(files) {
  if (!files || !files[0] || !state.currentChat) return;
  
  const file = files[0];
  const { type, id } = state.currentChat;
  
  // 创建进度条 UI（底部固定，不阻挡操作）
  const progressOverlay = createUploadProgressOverlay(file.name);
  const updateProgress = (percent, status) => {
    progressOverlay.querySelector('.upload-progress-fill').style.width = percent + '%';
    progressOverlay.querySelector('.upload-progress-text').textContent = Math.round(percent) + '%';
    if (status) {
      progressOverlay.querySelector('.upload-progress-title').textContent = status;
    }
  };
  
  try {
    // 计算哈希
    updateProgress(5, '计算哈希');
    const file_hash = await calculateSHA256(file);
    
    // 根据聊天类型确定文件类型
    let file_type;
    if (type === 'friend') {
      if (file.type.startsWith('image/')) file_type = 'friend_image';
      else if (file.type.startsWith('video/')) file_type = 'friend_video';
      else file_type = 'friend_document';
    } else {
      if (file.type.startsWith('image/')) file_type = 'group_image';
      else if (file.type.startsWith('video/')) file_type = 'group_video';
      else file_type = 'group_document';
    }
    
    // 请求上传凭证
    updateProgress(10, '准备上传');
    const uploadInfo = await api('/api/storage/upload/request', {
      method: 'POST',
      body: {
        file_type,
        storage_location: type === 'friend' ? 'friend_messages' : 'group_files',
        related_id: id,
        filename: file.name,
        file_size: file.size,
        content_type: file.type || 'application/octet-stream',
        file_hash,
        force_upload: false
      }
    });
    
    // 秒传检查
    if (uploadInfo.instant_upload) {
      updateProgress(100, '✓ 秒传成功');
      showToast('发送成功（秒传）', 'success');
      setTimeout(() => progressOverlay.remove(), 1500);
      await new Promise(r => setTimeout(r, 300));
      loadMessages();
      return;
    }
    
    // 预签名直传 MinIO
    if (uploadInfo.presigned_url) {
      // 将预签名URL转换为通过Nginx代理的URL
      let presignedUrl = uploadInfo.presigned_url;
      try {
        const url = new URL(presignedUrl);
        presignedUrl = BASE_URL + url.pathname + url.search;
      } catch (e) {}
      
      // PUT 直传 MinIO（真实进度）
      updateProgress(15, '上传中');
      await uploadToMinioWithProgress(presignedUrl, file, (percent) => {
        const realPercent = 15 + percent * 0.75; // 15% - 90%
        updateProgress(realPercent);
      });
      
      // 确认上传
      updateProgress(92, '完成中');
      const confirmResult = await api('/api/storage/upload/confirm', {
        method: 'POST',
        body: { file_key: uploadInfo.file_key }
      });
      
      updateProgress(100, '✓ 上传成功');
      console.log('上传确认响应:', confirmResult);
      showToast('发送成功', 'success');
      
      setTimeout(() => progressOverlay.remove(), 1200);
      await new Promise(r => setTimeout(r, 300));
      loadMessages();
      return;
    }
    
    // 分片上传（>= 5GB）
    if (uploadInfo.multipart_upload_id) {
      throw new Error('超大文件分片上传暂未实现前端界面');
    }
    
    throw new Error('不支持的上传模式');
    
  } catch (err) {
    console.error('文件上传错误:', err);
    updateProgress(0, '✕ 上传失败');
    progressOverlay.querySelector('.upload-progress-fill').style.background = '#e74c3c';
    progressOverlay.querySelector('.upload-progress-text').textContent = err.message || '未知错误';
    showToast(err.message || '上传失败', 'error');
    setTimeout(() => progressOverlay.remove(), 3000);
  }
  
  document.getElementById('chatFileInput').value = '';
}

// PUT 直传 MinIO（带进度）
function uploadToMinioWithProgress(url, file, onProgress) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    
    xhr.upload.addEventListener('progress', (e) => {
      if (e.lengthComputable) {
        const percent = (e.loaded / e.total) * 100;
        onProgress(percent);
      }
    });
    
    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        resolve();
      } else {
        reject(new Error(`上传失败: HTTP ${xhr.status}`));
      }
    });
    
    xhr.addEventListener('error', () => reject(new Error('网络错误')));
    xhr.addEventListener('abort', () => reject(new Error('上传取消')));
    
    xhr.open('PUT', url);
    xhr.setRequestHeader('Content-Type', file.type || 'application/octet-stream');
    xhr.send(file);
  });
}

// 获取或创建上传进度容器
function getUploadProgressContainer() {
  let container = document.getElementById('uploadProgressContainer');
  if (!container) {
    container = document.createElement('div');
    container.id = 'uploadProgressContainer';
    container.className = 'upload-progress-container';
    document.body.appendChild(container);
  }
  return container;
}

// 创建上传进度条（底部固定，不阻挡操作）
function createUploadProgressOverlay(filename) {
  const container = getUploadProgressContainer();
  const overlay = document.createElement('div');
  overlay.className = 'upload-progress-overlay';
  
  // 截断过长的文件名
  const displayName = filename.length > 30 ? filename.slice(0, 27) + '...' : filename;
  
  overlay.innerHTML = `
    <div class="upload-progress-modal">
      <div class="upload-progress-title">正在上传</div>
      <div class="upload-progress-filename" title="${filename}">${displayName}</div>
      <div class="upload-progress-bar">
        <div class="upload-progress-fill" style="width: 0%"></div>
      </div>
      <div class="upload-progress-text">0%</div>
    </div>
  `;
  
  container.appendChild(overlay);
  return overlay;
}

// 带进度的上传函数
function uploadWithProgress(url, formData, onProgress) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    
    xhr.upload.addEventListener('progress', (e) => {
      if (e.lengthComputable) {
        const percent = (e.loaded / e.total) * 100;
        onProgress(percent);
      }
    });
    
    xhr.addEventListener('load', () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          resolve(JSON.parse(xhr.responseText));
        } catch {
          resolve({});
        }
      } else {
        reject(new Error(xhr.responseText || '上传失败'));
      }
    });
    
    xhr.addEventListener('error', () => reject(new Error('网络错误')));
    xhr.addEventListener('abort', () => reject(new Error('上传取消')));
    
    xhr.open('POST', url);
    xhr.send(formData);
  });
}

// 输入框处理
function handleInputKeydown(e) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendMessage();
  }
}

function autoResizeInput(textarea) {
  textarea.style.height = 'auto';
  textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
}

// ==========================================
// 信息面板
// ==========================================

function toggleInfoPanel() {
  const panel = document.getElementById('infoPanel');
  const isVisible = panel.style.display !== 'none';
  
  if (isVisible) {
    panel.style.display = 'none';
  } else {
    renderInfoPanel();
    panel.style.display = 'flex';
  }
}

async function renderInfoPanel() {
  if (!state.currentChat) return;
  
  const { type, id, name } = state.currentChat;
  const content = document.getElementById('infoPanelContent');
  const title = document.getElementById('infoPanelTitle');
  
  if (type === 'friend') {
    title.textContent = '好友信息';
    content.innerHTML = `
      <div class="info-section" style="text-align: center;">
        <div class="info-avatar">👤</div>
        <div class="info-name">${name}</div>
        <div class="info-id">${id}</div>
      </div>
      <div class="info-section">
        <button class="btn-danger btn-block" onclick="removeFriend('${id}')">删除好友</button>
      </div>
    `;
  } else {
    title.textContent = '群聊信息';
    
    // 获取群详情
    const groupInfo = await api(`/api/groups/${id}`);
    const group = groupInfo.data || {};
    const members = await loadGroupMembers(id);
    const claims = decodeJwt(state.accessToken);
    const myMember = members.find(m => m.user_id === claims?.sub);
    const isAdmin = myMember?.role === 'owner' || myMember?.role === 'admin';
    const isOwner = myMember?.role === 'owner';
    
    // 群头像
    const groupAvatarHtml = group.group_avatar_url 
      ? `<img src="${group.group_avatar_url}" alt="群头像" class="avatar-img" style="width:80px;height:80px;border-radius:12px;" onerror="this.parentElement.innerHTML='👥'">`
      : '👥';
    
    content.innerHTML = `
      <div class="info-section" style="text-align: center;">
        <div class="info-avatar" style="font-size:50px;cursor:${isAdmin ? 'pointer' : 'default'}" ${isAdmin ? `onclick="showUploadGroupAvatarModal('${id}')" title="点击修改群头像"` : ''}>${groupAvatarHtml}</div>
        <div class="info-name">${group.group_name || name}</div>
        <div class="info-id">${members.length}人</div>
      </div>
      
      <div class="info-section">
        <h4>我的群昵称</h4>
        <div style="display:flex;gap:8px;align-items:center;">
          <input type="text" id="myGroupNickname" class="form-input" 
                 value="${myMember?.group_nickname || ''}" 
                 placeholder="设置群内昵称（不填则显示用户昵称）"
                 style="flex:1;">
          <button class="btn-primary" onclick="updateMyGroupNickname('${id}')">保存</button>
        </div>
      </div>
      
      ${isAdmin ? `
        <div class="info-section">
          <h4>群设置</h4>
          <div style="display:flex;gap:8px;align-items:center;margin-bottom:8px;">
            <input type="text" id="editGroupName" class="form-input" 
                   value="${group.group_name || ''}" 
                   placeholder="群名称"
                   style="flex:1;">
            <button class="btn-primary" onclick="updateGroupName('${id}')">修改</button>
          </div>
          <button class="btn-secondary btn-block" onclick="showUploadGroupAvatarModal('${id}')">上传群头像</button>
        </div>
      ` : ''}
      
      <div class="info-section">
        <h4>群成员</h4>
        <div class="member-list">
          ${members.map(m => {
            const memberAvatarHtml = m.user_avatar_url 
              ? `<img src="${m.user_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`
              : '👤';
            return `
              <div class="member-item">
                <div class="member-avatar">${memberAvatarHtml}</div>
                <div class="member-name">${m.group_nickname || m.user_nickname || m.user_id}</div>
                <span class="member-role ${m.role}">${m.role === 'owner' ? '群主' : m.role === 'admin' ? '管理员' : ''}</span>
              </div>
            `;
          }).join('')}
        </div>
      </div>
      
      ${isAdmin ? `
        <div class="info-section">
          <h4>群管理</h4>
          <button class="btn-secondary btn-block" onclick="showInviteMemberModal()">邀请成员</button>
          <button class="btn-secondary btn-block" onclick="showInviteCodeModal()" style="margin-top: 8px;">生成邀请码</button>
        </div>
      ` : ''}
      
      <div class="info-section">
        <button class="btn-danger btn-block" onclick="leaveGroup('${id}')">退出群聊</button>
      </div>
    `;
  }
}

async function leaveGroup(groupId) {
  if (!confirm('确定要退出这个群聊吗？')) return;
  
  try {
    await api(`/api/groups/${groupId}/leave`, { method: 'POST', body: {} });
    showToast('已退出群聊', 'success');
    closeChat();
    loadMyGroups();
    loadConversations();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 修改群名称
async function updateGroupName(groupId) {
  const nameInput = document.getElementById('editGroupName');
  const newName = nameInput?.value?.trim();
  
  if (!newName) {
    showToast('请输入群名称', 'error');
    return;
  }
  
  try {
    await api(`/api/groups/${groupId}`, {
      method: 'PUT',
      body: { group_name: newName }
    });
    showToast('群名称已更新', 'success');
    
    // 更新本地状态
    if (state.currentChat?.id === groupId) {
      state.currentChat.name = newName;
      document.getElementById('chatName').textContent = newName;
    }
    
    // 刷新群列表和会话列表
    await loadMyGroups();
    loadConversations();
    renderInfoPanel();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 修改我的群内昵称
async function updateMyGroupNickname(groupId) {
  const nicknameInput = document.getElementById('myGroupNickname');
  const nickname = nicknameInput?.value?.trim() || null;
  
  try {
    await api(`/api/groups/${groupId}/nickname`, {
      method: 'PUT',
      body: { nickname }
    });
    showToast(nickname ? '群昵称已更新' : '群昵称已清除', 'success');
    
    // 刷新群成员列表
    await loadGroupMembers(groupId);
    renderInfoPanel();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 显示上传群头像弹窗
function showUploadGroupAvatarModal(groupId) {
  const modal = document.createElement('div');
  modal.className = 'modal';
  modal.id = 'uploadGroupAvatarModal';
  modal.innerHTML = `
    <div class="modal-content" style="max-width: 400px;">
      <div class="modal-header">
        <h3>上传群头像</h3>
        <button class="modal-close" onclick="closeModal('uploadGroupAvatarModal')">&times;</button>
      </div>
      <div class="modal-body">
        <div style="text-align: center; margin-bottom: 16px;">
          <div id="groupAvatarPreview" style="width: 120px; height: 120px; border-radius: 12px; background: var(--bg-hover); margin: 0 auto; display: flex; align-items: center; justify-content: center; font-size: 48px; overflow: hidden;">
            👥
          </div>
        </div>
        <input type="file" id="groupAvatarFile" accept="image/*" style="display: none;" onchange="previewGroupAvatar(this)">
        <button class="btn-secondary btn-block" onclick="document.getElementById('groupAvatarFile').click()">选择图片</button>
        <p style="font-size: 12px; color: var(--text-secondary); margin-top: 8px; text-align: center;">支持 jpg、png、gif、webp，最大 10MB</p>
      </div>
      <div class="modal-footer">
        <button class="btn-secondary" onclick="closeModal('uploadGroupAvatarModal')">取消</button>
        <button class="btn-primary" onclick="uploadGroupAvatar('${groupId}')">上传</button>
      </div>
    </div>
  `;
  document.body.appendChild(modal);
  modal.style.display = 'flex';
}

// 预览群头像
function previewGroupAvatar(input) {
  const file = input.files[0];
  if (!file) return;
  
  const reader = new FileReader();
  reader.onload = (e) => {
    document.getElementById('groupAvatarPreview').innerHTML = `<img src="${e.target.result}" style="width:100%;height:100%;object-fit:cover;">`;
  };
  reader.readAsDataURL(file);
}

// 上传群头像
async function uploadGroupAvatar(groupId) {
  const fileInput = document.getElementById('groupAvatarFile');
  const file = fileInput?.files[0];
  
  if (!file) {
    showToast('请选择图片', 'error');
    return;
  }
  
  try {
    const formData = new FormData();
    formData.append('avatar', file);
    
    const result = await api(`/api/groups/${groupId}/avatar`, {
      method: 'POST',
      formData
    });
    
    showToast('群头像已更新', 'success');
    closeModal('uploadGroupAvatarModal');
    
    // 更新聊天头部头像
    if (result.data?.avatar_url) {
      document.getElementById('chatAvatar').innerHTML = `<img src="${result.data.avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👥'">`;
    }
    
    // 刷新群列表和会话列表
    await loadMyGroups();
    loadConversations();
    renderInfoPanel();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 文件功能
// ==========================================

// 加载我的文件
async function loadMyFiles() {
  try {
    const result = await api('/api/storage/files?page=1&limit=50');
    renderMyFiles(result.files || []);
  } catch (err) {
    console.error('加载文件失败:', err);
  }
}

function renderMyFiles(files) {
  const container = document.getElementById('myFilesList');
  
  if (files.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">📁</div>
        <p>暂无文件</p>
        <button class="btn-primary" onclick="switchFileTab('upload')">上传文件</button>
      </div>
    `;
    return;
  }
  
  container.innerHTML = files.map(f => {
    const icon = f.content_type?.startsWith('image/') ? '🖼️' :
                 f.content_type?.startsWith('video/') ? '🎬' :
                 f.content_type?.includes('pdf') ? '📄' : '📁';
    return `
      <div class="file-card" onclick="downloadFile('${f.file_uuid}')">
        <span class="file-icon">${icon}</span>
        <div class="file-info">
          <div class="file-name">${f.filename || '未命名'}</div>
          <div class="file-meta">${formatSize(f.file_size)} · ${formatTime(f.created_at)}</div>
        </div>
      </div>
    `;
  }).join('');
}

// 切换关联ID显示
function toggleRelatedId() {
  const location = document.getElementById('uploadLocation').value;
  const group = document.getElementById('relatedIdGroup');
  
  if (location === 'friend_messages') {
    group.style.display = 'block';
    document.querySelector('#relatedIdGroup label').textContent = '选择好友';
    document.getElementById('uploadRelatedId').innerHTML = state.friends.map(f => 
      `<option value="${f.friend_id}">${f.friend_nickname || f.friend_id}</option>`
    ).join('');
  } else if (location === 'group_files') {
    group.style.display = 'block';
    document.querySelector('#relatedIdGroup label').textContent = '选择群聊';
    document.getElementById('uploadRelatedId').innerHTML = state.groups.map(g => 
      `<option value="${g.group_id}">${g.group_name}</option>`
    ).join('');
  } else {
    group.style.display = 'none';
  }
}

// 处理文件选择
function handleFileSelect(files) {
  if (!files || !files[0]) return;
  
  const file = files[0];
  uploadFileToServer(file);
}

async function uploadFileToServer(file) {
  const progressContainer = document.getElementById('uploadProgress');
  const progressFill = document.getElementById('uploadProgressFill');
  const progressText = document.getElementById('uploadProgressText');
  
  progressContainer.style.display = 'block';
  progressFill.style.width = '0%';
  progressFill.style.background = 'linear-gradient(90deg, #667eea, #764ba2)';
  
  try {
    // 1. 计算哈希
    progressText.textContent = '计算文件哈希...';
    progressFill.style.width = '5%';
    
    const file_hash = await calculateSHA256(file);
    
    // 2. 确定参数
    const storage_location = document.getElementById('uploadLocation').value;
    const related_id = document.getElementById('uploadRelatedId')?.value;
    const force_upload = document.getElementById('forceUpload').checked;
    
    // 根据 storage_location 确定 file_type
    let file_type;
    if (storage_location === 'friend_messages') {
      if (file.type.startsWith('image/')) file_type = 'friend_image';
      else if (file.type.startsWith('video/')) file_type = 'friend_video';
      else file_type = 'friend_document';
    } else if (storage_location === 'group_files') {
      if (file.type.startsWith('image/')) file_type = 'group_image';
      else if (file.type.startsWith('video/')) file_type = 'group_video';
      else file_type = 'group_document';
    } else {
      if (file.type.startsWith('image/')) file_type = 'user_image';
      else if (file.type.startsWith('video/')) file_type = 'user_video';
      else file_type = 'user_document';
    }
    
    // 3. 请求上传
    progressText.textContent = '请求上传凭证...';
    progressFill.style.width = '10%';
    
    const body = {
      file_type,
      storage_location,
      filename: file.name,
      file_size: file.size,
      content_type: file.type || 'application/octet-stream',
      file_hash,
      force_upload
    };
    
    if (related_id && storage_location !== 'user_files') {
      body.related_id = related_id;
    }
    
    const uploadInfo = await api('/api/storage/upload/request', { method: 'POST', body });
    
    // 4. 秒传检查
    if (uploadInfo.instant_upload) {
      progressFill.style.width = '100%';
      progressText.textContent = '秒传成功！';
      showToast('文件上传成功（秒传）', 'success');
      loadMyFiles();
      return;
    }
    
    // 5. 使用 XMLHttpRequest 上传（带真实进度）
    progressText.textContent = '上传中 0%';
    progressFill.style.width = '15%';
    
    // 修正 upload_url 通过 Nginx 代理
    let uploadUrl = uploadInfo.upload_url;
    if (uploadUrl) {
      try {
        const url = new URL(uploadUrl);
        uploadUrl = BASE_URL + url.pathname + url.search;
      } catch (e) {}
    }
    
    const formData = new FormData();
    formData.append('file', file);
    
    await uploadWithProgress(uploadUrl, formData, (percent) => {
      const realPercent = 15 + percent * 0.8; // 15% - 95%
      progressFill.style.width = realPercent + '%';
      progressText.textContent = `上传中 ${Math.round(percent)}%`;
    });
    
    progressFill.style.width = '100%';
    progressText.textContent = '上传成功！';
    showToast('文件上传成功', 'success');
    loadMyFiles();
    
  } catch (err) {
    progressFill.style.width = '100%';
    progressFill.style.background = '#e74c3c';
    progressText.textContent = '上传失败: ' + err.message;
    showToast(err.message, 'error');
  }
}

async function downloadFile(uuid) {
  try {
    const result = await api(`/api/storage/file/${uuid}/presigned_url`, {
      method: 'POST',
      body: { operation: 'download' }
    });
    
    if (result.presigned_url) {
      window.open(result.presigned_url, '_blank');
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

async function previewFile(uuid) {
  try {
    const result = await api(`/api/storage/file/${uuid}/presigned_url`, {
      method: 'POST',
      body: { operation: 'download' }
    });
    
    if (result.presigned_url) {
      window.open(result.presigned_url, '_blank');
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 添加菜单
// ==========================================

function showAddMenu() {
  const btn = document.querySelector('.list-header .icon-btn');
  const rect = btn.getBoundingClientRect();
  const menu = document.getElementById('addMenu');
  
  menu.style.top = rect.bottom + 4 + 'px';
  menu.style.left = rect.left + 'px';
  menu.style.display = 'block';
  
  // 点击其他地方关闭
  setTimeout(() => {
    document.addEventListener('click', closeAddMenu, { once: true });
  }, 0);
}

function closeAddMenu() {
  document.getElementById('addMenu').style.display = 'none';
}

// ==========================================
// 搜索
// ==========================================

function handleSearch(query) {
  // 简单的本地搜索
  query = query.toLowerCase();
  
  const conversations = state.conversations.filter(c => 
    c.name.toLowerCase().includes(query) || c.id.toLowerCase().includes(query)
  );
  
  const container = document.getElementById('conversationList');
  
  if (conversations.length === 0 && query) {
    container.innerHTML = '<div class="empty-state"><p>未找到结果</p></div>';
    return;
  }
  
  if (!query) {
    renderConversations();
    return;
  }
  
  container.innerHTML = conversations.map(c => {
    const defaultIcon = c.type === 'group' ? '👥' : '👤';
    const avatarHtml = c.avatarUrl 
      ? `<img src="${c.avatarUrl}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='${defaultIcon}'">`
      : defaultIcon;
    return `
      <div class="conversation-item" onclick="openChat('${c.type}', '${c.id}', '${c.name}')">
        <div class="item-avatar">${avatarHtml}</div>
        <div class="item-info">
          <div class="item-name">${c.name}</div>
        </div>
      </div>
    `;
  }).join('');
}

// ==========================================
// 初始化
// ==========================================

async function initApp() {
  // 确保 Token 有效（会自动刷新过期的 Token）
  if (!await ensureAuth()) return;
  
  // 显示快速操作
  document.getElementById('quickActions').style.display = 'flex';
  
  // 加载用户头像
  try {
    const result = await api('/api/profile');
    if (result.data?.user_avatar_url) {
      document.getElementById('currentUserAvatar').innerHTML = `<img src="${result.data.user_avatar_url}" alt="">`;
    }
    state.currentUser = result.data;
  } catch {}
  
  // 加载数据
  await loadFriends();
  await loadMyGroups();
  loadConversations();
  
  // 连接 WebSocket
  await connectWebSocket();
  
  // 请求浏览器通知权限
  if (Notification.permission === 'default') {
    Notification.requestPermission();
  }
}

// 页面加载完成
document.addEventListener('DOMContentLoaded', () => {
  console.log('🚀 HuanVae Chat 已加载');
  
  if (state.accessToken) {
    closeModal('authModal');
    initApp();
  } else {
    openModal('authModal');
  }
});

// 拖拽上传
document.addEventListener('DOMContentLoaded', () => {
  const uploadArea = document.getElementById('uploadArea');
  if (uploadArea) {
    uploadArea.addEventListener('dragover', (e) => {
      e.preventDefault();
      uploadArea.style.borderColor = 'var(--primary)';
      uploadArea.style.background = 'var(--primary-light)';
    });
    
    uploadArea.addEventListener('dragleave', () => {
      uploadArea.style.borderColor = '';
      uploadArea.style.background = '';
    });
    
    uploadArea.addEventListener('drop', (e) => {
      e.preventDefault();
      uploadArea.style.borderColor = '';
      uploadArea.style.background = '';
      
      if (e.dataTransfer.files.length > 0) {
        handleFileSelect(e.dataTransfer.files);
      }
    });
  }
});
