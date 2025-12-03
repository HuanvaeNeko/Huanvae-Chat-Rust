/**
 * HuanVae Chat API 测试工具
 * 严格按照接口调取文档实现
 */

// ==========================================
// 全局状态
// ==========================================
const BASE = 'http://localhost:8080';

const state = {
  accessToken: localStorage.getItem('accessToken') || '',
  refreshToken: localStorage.getItem('refreshToken') || '',
  uploadedFiles: JSON.parse(localStorage.getItem('uploadedFiles') || '[]'),
  presignedUrls: {}
};

// ==========================================
// 工具函数
// ==========================================

// 通用 API 请求
async function api(path, { method = 'GET', token, body, formData } = {}) {
  const headers = {};
  
  if (body) {
    headers['Content-Type'] = 'application/json';
  }
  
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  
  const options = {
    method,
    headers,
    body: formData || (body ? JSON.stringify(body) : undefined)
  };
  
  const res = await fetch(`${BASE}${path}`, options);
  
  // 处理响应
  const contentType = res.headers.get('content-type') || '';
  let data;
  
  if (contentType.includes('application/json')) {
    data = await res.json().catch(() => ({}));
  } else {
    const text = await res.text().catch(() => '');
    data = text ? { error: text } : {};
  }
  
  if (!res.ok) {
    throw new Error(data.error || `请求失败: ${res.status}`);
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

// 格式化文件大小
function formatSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(2) + ' MB';
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}

// 格式化时间
function formatTime(isoStr) {
  if (!isoStr) return '';
  return new Date(isoStr).toLocaleString('zh-CN');
}

// 格式化 JSON 输出
function pretty(obj) {
  return JSON.stringify(obj, null, 2);
}

// 显示结果
function showResult(elementId, data, isError = false) {
  const el = document.getElementById(elementId);
  if (el) {
    el.textContent = typeof data === 'string' ? data : pretty(data);
    el.className = isError ? 'error' : 'success';
  }
}

// 显示 Toast 提示
function showToast(message, type = 'success') {
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  document.body.appendChild(toast);
  
  setTimeout(() => {
    toast.remove();
  }, 3000);
}

// 更新登录状态显示
function updateLoginStatus() {
  const dot = document.getElementById('statusDot');
  const text = document.getElementById('statusText');
  const info = document.getElementById('userInfo');
  
  if (state.accessToken) {
    const claims = decodeJwt(state.accessToken);
    dot.classList.add('online');
    text.textContent = '已登录';
    info.textContent = claims ? `用户: ${claims.sub}` : '';
  } else {
    dot.classList.remove('online');
    text.textContent = '未登录';
    info.textContent = '';
  }
}

// 保存 Token
function saveTokens(access, refresh) {
  state.accessToken = access;
  state.refreshToken = refresh;
  localStorage.setItem('accessToken', access);
  localStorage.setItem('refreshToken', refresh);
  updateLoginStatus();
}

// 清除 Token
function clearTokens() {
  state.accessToken = '';
  state.refreshToken = '';
  localStorage.removeItem('accessToken');
  localStorage.removeItem('refreshToken');
  updateLoginStatus();
}

// 计算文件 SHA-256 哈希（采样策略）
async function calculateSHA256(file) {
  const SAMPLE_SIZE = 10 * 1024 * 1024; // 10MB
  
  // 文件元信息
  const metadata = `${file.name}|${file.size}|${file.lastModified}|${file.type}`;
  const metadataBuffer = new TextEncoder().encode(metadata);
  
  let dataToHash;
  
  if (file.size <= SAMPLE_SIZE * 3) {
    // 小文件（< 30MB）：完整哈希
    dataToHash = await file.arrayBuffer();
  } else {
    // 大文件：采样哈希
    const chunks = [];
    
    // 开头 10MB
    const startBlob = file.slice(0, SAMPLE_SIZE);
    chunks.push(new Uint8Array(await startBlob.arrayBuffer()));
    
    // 中间 10MB
    const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
    const middleBlob = file.slice(middleStart, middleStart + SAMPLE_SIZE);
    chunks.push(new Uint8Array(await middleBlob.arrayBuffer()));
    
    // 结尾 10MB
    const endBlob = file.slice(file.size - SAMPLE_SIZE, file.size);
    chunks.push(new Uint8Array(await endBlob.arrayBuffer()));
    
    // 合并
    const totalLength = metadataBuffer.length + chunks.reduce((sum, c) => sum + c.length, 0);
    dataToHash = new Uint8Array(totalLength);
    let offset = 0;
    
    dataToHash.set(metadataBuffer, offset);
    offset += metadataBuffer.length;
    
    for (const chunk of chunks) {
      dataToHash.set(chunk, offset);
      offset += chunk.length;
    }
  }
  
  const hashBuffer = await crypto.subtle.digest('SHA-256', dataToHash);
  return Array.from(new Uint8Array(hashBuffer)).map(b => b.toString(16).padStart(2, '0')).join('');
}

// ==========================================
// 认证功能
// ==========================================

// 注册
async function register() {
  const user_id = document.getElementById('reg_user_id').value.trim();
  const nickname = document.getElementById('reg_nickname').value.trim();
  const password = document.getElementById('reg_password').value;
  const email = document.getElementById('reg_email').value.trim();
  
  if (!user_id || !nickname || !password) {
    showResult('regResult', { error: '请填写必要信息' }, true);
    return;
  }
  
  try {
    const body = { user_id, nickname, password };
    if (email) body.email = email;
    
    const result = await api('/api/auth/register', { method: 'POST', body });
    showResult('regResult', result);
    showToast('注册成功！');
  } catch (err) {
    showResult('regResult', { error: err.message }, true);
  }
}

// 登录
async function login() {
  const user_id = document.getElementById('login_user_id').value.trim();
  const password = document.getElementById('login_password').value;
  
  if (!user_id || !password) {
    showResult('loginResult', { error: '请输入用户ID和密码' }, true);
    return;
  }
  
  try {
    const result = await api('/api/auth/login', {
      method: 'POST',
      body: { user_id, password, device_info: navigator.userAgent }
    });
    
    saveTokens(result.access_token, result.refresh_token);
    showResult('loginResult', result);
    showToast('登录成功！');
  } catch (err) {
    showResult('loginResult', { error: err.message }, true);
  }
}

// 登出
async function logout() {
  if (!state.accessToken) {
    showToast('未登录', 'error');
    return;
  }
  
  try {
    await api('/api/auth/logout', { method: 'POST', token: state.accessToken });
    clearTokens();
    showResult('loginResult', { message: '已登出' });
    showToast('已登出');
  } catch (err) {
    clearTokens();
    showToast('已登出');
  }
}

// 刷新 Token
async function refreshToken() {
  if (!state.refreshToken) {
    showToast('无刷新令牌', 'error');
    return;
  }
  
  try {
    const result = await api('/api/auth/refresh', {
      method: 'POST',
      body: { refresh_token: state.refreshToken }
    });
    
    state.accessToken = result.access_token;
    localStorage.setItem('accessToken', result.access_token);
    updateLoginStatus();
    showResult('loginResult', result);
    showToast('Token 已刷新');
  } catch (err) {
    showResult('loginResult', { error: err.message }, true);
  }
}

// 设备列表
async function listDevices() {
  if (!state.accessToken) {
    showToast('请先登录', 'error');
    return;
  }
  
  try {
    const result = await api('/api/auth/devices', { token: state.accessToken });
    const container = document.getElementById('devicesList');
    
    if (result.devices && result.devices.length > 0) {
      container.innerHTML = result.devices.map(d => `
        <div class="list-item">
          <div class="info">
            <div class="name">${d.device_info || '未知设备'}</div>
            <div class="meta">
              ID: ${d.device_id.substring(0, 8)}... · 
              最后活跃: ${formatTime(d.last_used_at)}
              ${d.is_current ? '<span class="badge green">当前</span>' : ''}
            </div>
          </div>
          <div class="actions">
            <button onclick="revokeDevice('${d.device_id}')" class="danger">移除</button>
          </div>
        </div>
      `).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><p>无设备</p></div>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 移除设备
async function revokeDevice(deviceId) {
  try {
    await api(`/api/auth/devices/${deviceId}`, { method: 'DELETE', token: state.accessToken });
    showToast('设备已移除');
    listDevices();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 好友功能
// ==========================================

// 发送好友请求
async function submitFriendRequest() {
  if (!state.accessToken) {
    showResult('friendRequestResult', { error: '请先登录' }, true);
    return;
  }
  
  const claims = decodeJwt(state.accessToken);
  if (!claims || !claims.sub) {
    showResult('friendRequestResult', { error: 'Token 无效' }, true);
    return;
  }
  
  const target_user_id = document.getElementById('friend_target_id').value.trim();
  if (!target_user_id) {
    showResult('friendRequestResult', { error: '请输入目标用户ID' }, true);
    return;
  }
  
  const reason = document.getElementById('friend_reason').value.trim();
  
  try {
    const body = {
      user_id: claims.sub,
      target_user_id,
      request_time: new Date().toISOString()
    };
    if (reason) body.reason = reason;
    
    const result = await api('/api/friends/requests', {
      method: 'POST',
      token: state.accessToken,
      body
    });
    
    showResult('friendRequestResult', result);
    showToast('好友请求已发送');
  } catch (err) {
    showResult('friendRequestResult', { error: err.message }, true);
  }
}

// 待处理请求列表
async function listPendingRequests() {
  if (!state.accessToken) return;
  
  try {
    const result = await api('/api/friends/requests/pending', { token: state.accessToken });
    const container = document.getElementById('pendingList');
    
    if (result.items && result.items.length > 0) {
      container.innerHTML = result.items.map(item => `
        <div class="list-item">
          <div class="info">
            <div class="name">${item.request_user_id}</div>
            <div class="meta">${item.request_message || '无附言'} · ${formatTime(item.request_time)}</div>
          </div>
          <div class="actions">
            <button onclick="approveFriendRequest('${item.request_user_id}')" class="success">同意</button>
            <button onclick="rejectFriendRequest('${item.request_user_id}')" class="danger">拒绝</button>
          </div>
        </div>
      `).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><p>无待处理请求</p></div>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 已发送请求列表
async function listSentRequests() {
  if (!state.accessToken) return;
  
  try {
    const result = await api('/api/friends/requests/sent', { token: state.accessToken });
    const container = document.getElementById('sentList');
    
    if (result.items && result.items.length > 0) {
      container.innerHTML = result.items.map(item => `
        <div class="list-item">
          <div class="info">
            <div class="name">${item.sent_to_user_id}</div>
            <div class="meta">${item.sent_message || ''} · ${formatTime(item.sent_time)}</div>
          </div>
          <span class="badge orange">等待中</span>
        </div>
      `).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><p>无已发送请求</p></div>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 好友列表
async function listFriends() {
  if (!state.accessToken) return;
  
  try {
    const result = await api('/api/friends', { token: state.accessToken });
    const container = document.getElementById('friendsList');
    
    if (result.items && result.items.length > 0) {
      container.innerHTML = result.items.map(item => `
        <div class="list-item">
          <div class="info">
            <div class="name">${item.friend_id}</div>
            <div class="meta">${item.friend_nickname || ''} · 添加于 ${formatTime(item.add_time)}</div>
          </div>
          <div class="actions">
            <button onclick="document.getElementById('chat_friend_id').value='${item.friend_id}';loadMessages()" class="secondary">聊天</button>
            <button onclick="removeFriend('${item.friend_id}')" class="danger">删除</button>
          </div>
        </div>
      `).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><p>暂无好友</p></div>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 同意好友请求
async function approveFriendRequest(applicantId) {
  const claims = decodeJwt(state.accessToken);
  
  try {
    await api('/api/friends/requests/approve', {
      method: 'POST',
      token: state.accessToken,
      body: {
        user_id: claims.sub,
        applicant_user_id: applicantId,
        approved_time: new Date().toISOString()
      }
    });
    
    showToast('已同意好友请求');
    listPendingRequests();
    listFriends();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 拒绝好友请求
async function rejectFriendRequest(applicantId) {
  const claims = decodeJwt(state.accessToken);
  
  try {
    await api('/api/friends/requests/reject', {
      method: 'POST',
      token: state.accessToken,
      body: {
        user_id: claims.sub,
        applicant_user_id: applicantId
      }
    });
    
    showToast('已拒绝好友请求');
    listPendingRequests();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 删除好友
async function removeFriend(friendId) {
  if (!confirm(`确定要删除好友 ${friendId} 吗？`)) return;
  
  const claims = decodeJwt(state.accessToken);
  
  try {
    await api('/api/friends/remove', {
      method: 'POST',
      token: state.accessToken,
      body: {
        user_id: claims.sub,
        friend_user_id: friendId,
        remove_time: new Date().toISOString()
      }
    });
    
    showToast('已删除好友');
    listFriends();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// ==========================================
// 消息功能
// ==========================================

// 切换文件上传区域显示
function toggleFileUpload() {
  const msgType = document.getElementById('msg_type').value;
  const fileSection = document.getElementById('file_upload_section');
  const btnUploadAndSend = document.getElementById('btnUploadAndSend');
  const btnSendText = document.querySelector('button[onclick="sendMessage()"]');
  
  if (msgType === 'text') {
    fileSection.style.display = 'none';
    btnUploadAndSend.style.display = 'none';
    btnSendText.style.display = 'inline-block';
  } else {
    fileSection.style.display = 'block';
    btnUploadAndSend.style.display = 'inline-block';
    btnSendText.style.display = 'none';
  }
}

// 上传文件并发送消息（好友文件自动消息功能）
async function uploadAndSendFile() {
  if (!state.accessToken) {
    showResult('sendMsgResult', { error: '请先登录' }, true);
    return;
  }
  
  const receiver_id = document.getElementById('msg_receiver_id').value.trim();
  const fileInput = document.getElementById('msg_file_input');
  const file = fileInput.files[0];
  const message_type = document.getElementById('msg_type').value;
  
  if (!receiver_id) {
    showResult('sendMsgResult', { error: '请填写接收者ID' }, true);
    return;
  }
  
  if (!file) {
    showResult('sendMsgResult', { error: '请选择要发送的文件' }, true);
    return;
  }
  
  const progressBar = document.getElementById('msgUploadProgress');
  const progressFill = document.getElementById('msgUploadProgressFill');
  const progressText = document.getElementById('msgUploadProgressText');
  
  try {
    // 1. 计算文件哈希
    progressBar.style.display = 'block';
    progressFill.style.width = '10%';
    progressText.textContent = '计算文件哈希...';
    
    const file_hash = await calculateSHA256(file);
    
    // 2. 确定文件类型
    const file_type = message_type === 'image' ? 'user_image' : 
                      message_type === 'video' ? 'user_video' : 'user_document';
    
    // 3. 请求上传
    progressFill.style.width = '20%';
    progressText.textContent = '请求上传...';
    
    const uploadInfo = await api('/api/storage/upload/request', {
      method: 'POST',
      token: state.accessToken,
      body: {
        file_type,
        storage_location: 'friend_messages',
        related_id: receiver_id,
        filename: file.name,
        file_size: file.size,
        content_type: file.type || 'application/octet-stream',
        file_hash,
        force_upload: false
      }
    });
    
    // 4. 检查秒传
    if (uploadInfo.instant_upload) {
      progressFill.style.width = '100%';
      progressText.textContent = '秒传成功！消息已发送';
      showResult('sendMsgResult', uploadInfo);
      showToast(`文件已秒传发送给 ${receiver_id}`);
      
      // 刷新聊天
      if (document.getElementById('chat_friend_id').value === receiver_id) {
        loadMessages();
      }
      return;
    }
    
    // 5. 上传文件
    progressFill.style.width = '50%';
    progressText.textContent = '上传文件中...';
    
    const formData = new FormData();
    formData.append('file', file);
    
    const uploadResult = await fetch(uploadInfo.upload_url, {
      method: 'POST',
      body: formData
    });
    
    if (!uploadResult.ok) {
      throw new Error('上传失败');
    }
    
    const result = await uploadResult.json();
    
    // 6. 上传完成，消息已自动发送
    progressFill.style.width = '100%';
    
    if (result.message_uuid) {
      progressText.textContent = '上传成功！消息已自动发送';
      showToast(`文件消息已发送给 ${receiver_id}`);
    } else {
      progressText.textContent = '上传成功！';
    }
    
    showResult('sendMsgResult', result);
    
    // 清空文件输入
    fileInput.value = '';
    
    // 刷新聊天
    if (document.getElementById('chat_friend_id').value === receiver_id) {
      loadMessages();
    }
    
  } catch (err) {
    progressFill.style.width = '0%';
    progressText.textContent = '发送失败: ' + err.message;
    showResult('sendMsgResult', { error: err.message }, true);
  }
}

// 发送文本消息
async function sendMessage() {
  if (!state.accessToken) {
    showResult('sendMsgResult', { error: '请先登录' }, true);
    return;
  }
  
  const receiver_id = document.getElementById('msg_receiver_id').value.trim();
  const message_content = document.getElementById('msg_content').value.trim();
  const message_type = document.getElementById('msg_type').value;
  
  if (!receiver_id || !message_content) {
    showResult('sendMsgResult', { error: '请填写接收者和内容' }, true);
    return;
  }
  
  try {
    const body = { receiver_id, message_content, message_type };
    
    const result = await api('/api/messages', {
      method: 'POST',
      token: state.accessToken,
      body
    });
    
    showResult('sendMsgResult', result);
    showToast('消息已发送');
    
    // 清空输入框
    document.getElementById('msg_content').value = '';
    
    // 如果正在查看与该好友的对话，刷新
    if (document.getElementById('chat_friend_id').value === receiver_id) {
      loadMessages();
    }
  } catch (err) {
    showResult('sendMsgResult', { error: err.message }, true);
  }
}

// 加载消息列表
async function loadMessages() {
  if (!state.accessToken) return;
  
  const friend_id = document.getElementById('chat_friend_id').value.trim();
  if (!friend_id) {
    showToast('请输入好友ID', 'error');
    return;
  }
  
  try {
    const result = await api(`/api/messages?friend_id=${encodeURIComponent(friend_id)}&limit=50`, {
      token: state.accessToken
    });
    
    const container = document.getElementById('messagesList');
    const claims = decodeJwt(state.accessToken);
    const myId = claims ? claims.sub : '';
    
    if (result.messages && result.messages.length > 0) {
      container.innerHTML = result.messages.map(msg => {
        const isSent = msg.sender_id === myId;
        const hasFile = msg.file_uuid && msg.file_uuid !== 'null';
        const isMedia = hasFile && (msg.message_type === 'image' || msg.message_type === 'video');
        
        return `
          <div class="message-item ${isSent ? 'sent' : 'received'}">
            <div class="message-header">
              ${isSent ? '我' : msg.sender_id} · ${formatTime(msg.send_time)}
            </div>
            <div class="message-content">
              ${msg.message_type !== 'text' ? `[${msg.message_type}] ` : ''}${msg.message_content}
              ${hasFile ? `
                <div id="preview-${msg.message_uuid}" style="margin-top: 8px;">
                  <button onclick="previewMessageFile('${msg.file_uuid}', '${msg.message_type}', '${msg.message_uuid}')" 
                    style="padding: 4px 10px; font-size: 0.75rem; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;">
                    ${isMedia ? '👁️ 预览' : '📥 下载'}
                  </button>
                </div>
              ` : ''}
            </div>
            <div style="margin-top: 6px; font-size: 0.7rem; opacity: 0.6;">
              ${msg.message_uuid.substring(0, 8)}...
              <button onclick="document.getElementById('op_message_uuid').value='${msg.message_uuid}'" style="padding: 2px 6px; font-size: 0.65rem;">选择</button>
            </div>
          </div>
        `;
      }).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><div class="icon">💭</div><p>暂无消息</p></div>';
    }
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 预览消息中的文件
async function previewMessageFile(fileUuid, messageType, messageUuid) {
  if (!state.accessToken || !fileUuid) return;
  
  const previewContainer = document.getElementById(`preview-${messageUuid}`);
  if (!previewContainer) return;
  
  // 显示加载状态
  previewContainer.innerHTML = '<span style="color: var(--text-secondary);">加载中...</span>';
  
  try {
    // 获取好友文件预签名URL
    const result = await api(`/api/storage/friends-file/${fileUuid}/presigned-url`, {
      method: 'POST',
      token: state.accessToken,
      body: { operation: 'download' }
    });
    
    const url = result.presigned_url;
    const contentType = result.content_type || '';
    
    // 根据类型显示预览
    if (contentType.startsWith('image/')) {
      previewContainer.innerHTML = `
        <img src="${url}" alt="图片" style="max-width: 100%; max-height: 300px; border-radius: 8px; margin-top: 8px; cursor: pointer;" 
          onclick="window.open('${url}', '_blank')" />
        <div style="margin-top: 4px;">
          <a href="${url}" download style="font-size: 0.75rem; color: var(--accent-blue);">下载原图</a>
        </div>
      `;
    } else if (contentType.startsWith('video/')) {
      previewContainer.innerHTML = `
        <video controls style="max-width: 100%; max-height: 300px; border-radius: 8px; margin-top: 8px;">
          <source src="${url}" type="${contentType}">
          您的浏览器不支持视频播放
        </video>
        <div style="margin-top: 4px;">
          <a href="${url}" download style="font-size: 0.75rem; color: var(--accent-blue);">下载视频</a>
        </div>
      `;
    } else {
      // 其他文件类型直接提供下载
      previewContainer.innerHTML = `
        <a href="${url}" download style="display: inline-block; padding: 6px 12px; background: var(--accent-green); color: white; border-radius: 4px; text-decoration: none; font-size: 0.75rem;">
          📥 下载文件
        </a>
      `;
    }
  } catch (err) {
    previewContainer.innerHTML = `<span style="color: var(--accent-red); font-size: 0.75rem;">预览失败: ${err.message}</span>`;
  }
}

// 删除消息
async function deleteMessage() {
  const message_uuid = document.getElementById('op_message_uuid').value.trim();
  if (!message_uuid) {
    showResult('msgOpResult', { error: '请输入消息UUID' }, true);
    return;
  }
  
  try {
    const result = await api('/api/messages/delete', {
      method: 'DELETE',
      token: state.accessToken,
      body: { message_uuid }
    });
    
    showResult('msgOpResult', result);
    showToast('消息已删除');
    loadMessages();
  } catch (err) {
    showResult('msgOpResult', { error: err.message }, true);
  }
}

// 撤回消息
async function recallMessage() {
  const message_uuid = document.getElementById('op_message_uuid').value.trim();
  if (!message_uuid) {
    showResult('msgOpResult', { error: '请输入消息UUID' }, true);
    return;
  }
  
  try {
    const result = await api('/api/messages/recall', {
      method: 'POST',
      token: state.accessToken,
      body: { message_uuid }
    });
    
    showResult('msgOpResult', result);
    showToast('消息已撤回');
    loadMessages();
  } catch (err) {
    showResult('msgOpResult', { error: err.message }, true);
  }
}

// ==========================================
// 个人资料功能
// ==========================================

// 加载个人资料
async function loadProfile() {
  if (!state.accessToken) {
    showResult('profileResult', { error: '请先登录' }, true);
    return;
  }
  
  try {
    const result = await api('/api/profile', { token: state.accessToken });
    showResult('profileResult', result);
    
    if (result.data) {
      document.getElementById('profile_email').value = result.data.user_email || '';
      document.getElementById('profile_signature').value = result.data.user_signature || '';
    }
  } catch (err) {
    showResult('profileResult', { error: err.message }, true);
  }
}

// 更新资料
async function updateProfile() {
  if (!state.accessToken) return;
  
  const email = document.getElementById('profile_email').value.trim();
  const signature = document.getElementById('profile_signature').value.trim();
  
  if (!email && !signature) {
    showToast('请至少填写一项', 'error');
    return;
  }
  
  try {
    const body = {};
    if (email) body.email = email;
    if (signature) body.signature = signature;
    
    await api('/api/profile', {
      method: 'PUT',
      token: state.accessToken,
      body
    });
    
    showToast('资料已更新');
    loadProfile();
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 修改密码
async function changePassword() {
  const old_password = document.getElementById('old_password').value;
  const new_password = document.getElementById('new_password').value;
  
  if (!old_password || !new_password) {
    showToast('请填写密码', 'error');
    return;
  }
  
  try {
    await api('/api/profile/password', {
      method: 'PUT',
      token: state.accessToken,
      body: { old_password, new_password }
    });
    
    showToast('密码已修改');
    document.getElementById('old_password').value = '';
    document.getElementById('new_password').value = '';
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 上传头像
async function uploadAvatar() {
  const fileInput = document.getElementById('avatar_file');
  const file = fileInput.files[0];
  
  if (!file) {
    showResult('avatarResult', { error: '请选择文件' }, true);
    return;
  }
  
  try {
    const formData = new FormData();
    formData.append('avatar', file);
    
    const result = await api('/api/profile/avatar', {
      method: 'POST',
      token: state.accessToken,
      formData
    });
    
    showResult('avatarResult', result);
    showToast('头像已上传');
  } catch (err) {
    showResult('avatarResult', { error: err.message }, true);
  }
}

// ==========================================
// 文件存储功能
// ==========================================

// 切换标签页
function switchTab(tabName) {
  document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
  
  event.target.classList.add('active');
  document.getElementById(`tab-${tabName}`).classList.add('active');
}

// 存储位置改变时显示/隐藏关联ID输入
document.addEventListener('DOMContentLoaded', () => {
  const storageSelect = document.getElementById('upload_storage_location');
  const relatedIdRow = document.getElementById('related_id_row');
  
  if (storageSelect && relatedIdRow) {
    storageSelect.addEventListener('change', () => {
      relatedIdRow.style.display = storageSelect.value === 'friend_messages' ? 'flex' : 'none';
    });
  }
  
  const expiresSelect = document.getElementById('friend_file_expires');
  const extendedRow = document.getElementById('extended_hours_row');
  
  if (expiresSelect && extendedRow) {
    expiresSelect.addEventListener('change', () => {
      extendedRow.style.display = expiresSelect.value === 'extended' ? 'flex' : 'none';
    });
  }
});

// 显示上传进度
function showUploadProgress(percent, text) {
  const progressBar = document.getElementById('uploadProgress');
  const progressFill = document.getElementById('uploadProgressFill');
  const progressText = document.getElementById('uploadProgressText');
  
  progressBar.style.display = 'block';
  progressFill.style.width = `${percent}%`;
  progressText.textContent = text;
}

// 上传文件
async function uploadFile() {
  const fileInput = document.getElementById('upload_file');
  const file = fileInput.files[0];
  
  if (!file) {
    showResult('uploadResult', { error: '请选择文件' }, true);
    return;
  }
  
  if (!state.accessToken) {
    showResult('uploadResult', { error: '请先登录' }, true);
    return;
  }
  
  const file_type = document.getElementById('upload_file_type').value;
  const storage_location = document.getElementById('upload_storage_location').value;
  const related_id = document.getElementById('upload_related_id').value.trim();
  const force_upload = document.getElementById('force_upload').checked;
  
  if (storage_location === 'friend_messages' && !related_id) {
    showResult('uploadResult', { error: '好友消息存储需要填写好友ID' }, true);
    return;
  }
  
  try {
    // 1. 计算哈希
    showUploadProgress(10, '计算文件哈希...');
    const file_hash = await calculateSHA256(file);
    
    // 2. 请求上传
    showUploadProgress(20, '请求上传...');
    const requestBody = {
      file_type,
      storage_location,
      filename: file.name,
      file_size: file.size,
      content_type: file.type || 'application/octet-stream',
      file_hash,
      force_upload
    };
    
    if (related_id) requestBody.related_id = related_id;
    
    const uploadInfo = await api('/api/storage/upload/request', {
      method: 'POST',
      token: state.accessToken,
      body: requestBody
    });
    
    // 3. 检查秒传
    if (uploadInfo.instant_upload) {
      showUploadProgress(100, '秒传成功！');
      showResult('uploadResult', uploadInfo);
      showToast('秒传成功！');
      
      // 保存文件信息
      saveUploadedFile({
        filename: file.name,
        file_url: uploadInfo.existing_file_url,
        file_key: uploadInfo.file_key,
        file_size: file.size,
        instant: true,
        timestamp: new Date().toISOString()
      });
      
      return;
    }
    
    // 4. 实际上传
    showUploadProgress(40, '上传文件...');
    
    const formData = new FormData();
    formData.append('file', file);
    
    const uploadResult = await fetch(uploadInfo.upload_url, {
      method: 'POST',
      body: formData
    });
    
    if (!uploadResult.ok) {
      throw new Error('上传失败');
    }
    
    const result = await uploadResult.json();
    
    // 好友文件：检查自动消息
    if (result.message_uuid) {
      showUploadProgress(100, '上传成功！消息已自动发送');
      showToast(`文件消息已自动发送给 ${related_id}`);
      
      // 刷新聊天界面（如果正在与该好友聊天）
      const chatFriendId = document.getElementById('chat_friend_id')?.value;
      if (chatFriendId === related_id) {
        loadMessages();
      }
    } else {
      showUploadProgress(100, '上传成功！');
      showToast('上传成功！');
    }
    
    showResult('uploadResult', result);
    
    // 保存文件信息
    saveUploadedFile({
      filename: file.name,
      file_url: result.file_url,
      file_key: result.file_key,
      file_size: result.file_size,
      message_uuid: result.message_uuid,
      message_send_time: result.message_send_time,
      instant: false,
      timestamp: new Date().toISOString()
    });
    
  } catch (err) {
    showResult('uploadResult', { error: err.message }, true);
    showUploadProgress(0, '上传失败');
  }
}

// 保存已上传文件
function saveUploadedFile(fileInfo) {
  state.uploadedFiles.unshift(fileInfo);
  if (state.uploadedFiles.length > 20) {
    state.uploadedFiles = state.uploadedFiles.slice(0, 20);
  }
  localStorage.setItem('uploadedFiles', JSON.stringify(state.uploadedFiles));
}

// 获取文件UUID
function extractUuid(url) {
  if (!url) return '';
  const parts = url.split('/');
  return parts[parts.length - 1] || '';
}

// 列出我的文件
async function listMyFiles() {
  if (!state.accessToken) {
    showResult('myFilesResult', { error: '请先登录' }, true);
    return;
  }
  
  try {
    const result = await api('/api/storage/files?page=1&limit=20&sort_by=created_at&sort_order=desc', {
      token: state.accessToken
    });
    
    showResult('myFilesResult', result);
    
    const container = document.getElementById('myFilesList');
    
    if (result.files && result.files.length > 0) {
      container.innerHTML = result.files.map(f => {
        const icon = f.content_type?.startsWith('image/') ? '🖼️' :
                     f.content_type?.startsWith('video/') ? '🎬' :
                     f.content_type?.startsWith('application/pdf') ? '📄' : '📁';
        return `
          <div class="file-card">
            <div class="icon">${icon}</div>
            <div class="name">${f.original_filename || f.file_key}</div>
            <div class="size">${formatSize(f.file_size)}</div>
            <button onclick="getPersonalFileUrl('${f.file_uuid}')" style="margin-top: 8px; font-size: 0.75rem;">获取URL</button>
          </div>
        `;
      }).join('');
    } else {
      container.innerHTML = '<div class="empty-state"><div class="icon">📂</div><p>暂无文件</p></div>';
    }
  } catch (err) {
    showResult('myFilesResult', { error: err.message }, true);
  }
}

// 获取个人文件预签名URL
async function getPersonalFileUrl(uuid) {
  try {
    const result = await api(`/api/storage/file/${uuid}/presigned_url`, {
      method: 'POST',
      token: state.accessToken,
      body: { operation: 'download' }
    });
    
    showResult('myFilesResult', result);
    showToast('已获取预签名URL');
    
    // 复制到剪贴板
    navigator.clipboard.writeText(result.presigned_url).then(() => {
      showToast('URL已复制到剪贴板');
    });
  } catch (err) {
    showToast(err.message, 'error');
  }
}

// 获取好友文件预签名URL
async function getFriendFileUrl() {
  const uuid = document.getElementById('friend_file_uuid').value.trim();
  if (!uuid) {
    showResult('friendFileResult', { error: '请输入文件UUID' }, true);
    return;
  }
  
  if (!state.accessToken) {
    showResult('friendFileResult', { error: '请先登录' }, true);
    return;
  }
  
  const expiresType = document.getElementById('friend_file_expires').value;
  
  try {
    let endpoint, body;
    
    if (expiresType === 'extended') {
      const hours = parseInt(document.getElementById('extended_hours').value) || 24;
      const seconds = Math.min(Math.max(hours * 3600, 10800), 604800);
      
      endpoint = `/api/storage/friends-file/${uuid}/presigned-url/extended`;
      body = { operation: 'download', estimated_download_time: seconds };
    } else {
      endpoint = `/api/storage/friends-file/${uuid}/presigned-url`;
      body = { operation: 'download' };
    }
    
    const result = await api(endpoint, {
      method: 'POST',
      token: state.accessToken,
      body
    });
    
    showResult('friendFileResult', result);
    
    // 预览文件
    previewFile(result.presigned_url, result.content_type);
  } catch (err) {
    showResult('friendFileResult', { error: err.message }, true);
  }
}

// 预览文件
function previewFile(url, contentType) {
  const container = document.getElementById('filePreview');
  
  if (!url) {
    container.innerHTML = '<div class="empty-state"><div class="icon">🖼️</div><p>获取预签名URL后可预览</p></div>';
    return;
  }
  
  if (contentType?.startsWith('image/')) {
    container.innerHTML = `
      <img src="${url}" alt="预览" style="max-width: 100%; max-height: 400px; border-radius: 8px;" />
      <div style="margin-top: 12px;">
        <a href="${url}" download style="color: var(--accent-blue);">下载图片</a>
      </div>
    `;
  } else if (contentType?.startsWith('video/')) {
    container.innerHTML = `
      <video controls style="max-width: 100%; max-height: 400px; border-radius: 8px;">
        <source src="${url}" type="${contentType}">
      </video>
      <div style="margin-top: 12px;">
        <a href="${url}" download style="color: var(--accent-blue);">下载视频</a>
      </div>
    `;
  } else if (contentType === 'application/pdf') {
    container.innerHTML = `
      <iframe src="${url}" style="width: 100%; height: 400px; border: none; border-radius: 8px;"></iframe>
      <div style="margin-top: 12px;">
        <a href="${url}" download style="color: var(--accent-blue);">下载PDF</a>
      </div>
    `;
  } else {
    container.innerHTML = `
      <div class="empty-state">
        <div class="icon">📁</div>
        <p>该文件类型不支持预览</p>
        <a href="${url}" download style="display: inline-block; margin-top: 12px; padding: 10px 20px; background: var(--accent-green); color: white; border-radius: 8px; text-decoration: none;">下载文件</a>
      </div>
    `;
  }
}

// ==========================================
// 初始化
// ==========================================

document.addEventListener('DOMContentLoaded', () => {
  console.log('🚀 HuanVae Chat API 测试工具已加载');
  updateLoginStatus();
  
  // 初始化消息类型切换
  toggleFileUpload();
});
