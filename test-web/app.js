const $ = (id) => document.getElementById(id);
const pretty = (obj) => JSON.stringify(obj, null, 2);

const state = {
  apiBase: localStorage.getItem('apiBase') || 'http://localhost:8080/api/auth',
  friendsBase: localStorage.getItem('friendsApiBase') || 'http://localhost:8080/api/friends',
  messagesBase: localStorage.getItem('messagesApiBase') || 'http://localhost:8080/api/messages',
  storageBase: localStorage.getItem('storageApiBase') || 'http://localhost:8080/api/storage',
  uploadedFiles: JSON.parse(localStorage.getItem('uploadedFiles') || '[]'),
};

// 使用TokenManager管理token和Service Worker
// tokenManager 在 token-manager.js 中定义，已作为全局变量可用

async function init() {
  $('apiBase').value = state.apiBase;
  $('friendsApiBase').value = state.friendsBase;
  $('messagesApiBase').value = state.messagesBase;
  $('storageApiBase').value = state.storageBase;
  renderLocalState();
  
  // 初始化TokenManager（会自动注册Service Worker）
  await tokenManager.initialize();
  $('saveBase').onclick = () => {
    state.apiBase = $('apiBase').value.trim() || state.apiBase;
    localStorage.setItem('apiBase', state.apiBase);
    renderLocalState();
  };
  $('saveFriendsBase').onclick = () => {
    state.friendsBase = $('friendsApiBase').value.trim() || state.friendsBase;
    localStorage.setItem('friendsApiBase', state.friendsBase);
    renderLocalState();
  };
  $('saveMessagesBase').onclick = () => {
    state.messagesBase = $('messagesApiBase').value.trim() || state.messagesBase;
    localStorage.setItem('messagesApiBase', state.messagesBase);
    renderLocalState();
  };
  $('saveStorageBase').onclick = () => {
    state.storageBase = $('storageApiBase').value.trim() || state.storageBase;
    localStorage.setItem('storageApiBase', state.storageBase);
    renderLocalState();
  };

  $('btnRegister').onclick = register;
  $('btnLogin').onclick = login;
  $('btnShowProfile').onclick = showProfileFromAccessToken;
  $('btnRefreshToken').onclick = refreshAccessToken;
  $('btnListDevices').onclick = listDevices;
  $('btnClear').onclick = () => {
    tokenManager.clearTokens();
    renderLocalState();
    $('profile').textContent = '';
    $('devices').innerHTML = '';
  };

  // Friends
  $('btnSubmitFriend').onclick = submitFriendRequest;
  $('btnListPending').onclick = listPendingRequests;
  $('btnListSent').onclick = listSentRequests;
  $('btnListFriends').onclick = listFriends;
  $('btnRemoveFriendDirect').onclick = () => {
    const id = $('remove_friend_id').value.trim();
    const reason = $('remove_friend_reason').value.trim();
    if (!id) { $('removeResFmt').textContent = '请输入好友ID'; return; }
    removeFriend(id, reason);
  };

  // Messages
  $('btnSendMessage').onclick = sendMessage;
  $('btnGetMessages').onclick = getMessages;
  $('btnDeleteMessage').onclick = () => {
    const uuid = $('delete_message_uuid').value.trim();
    if (!uuid) { $('msgDeleteResFmt').textContent = '请输入消息UUID'; return; }
    deleteMessage(uuid);
  };
  $('btnRecallMessage').onclick = () => {
    const uuid = $('recall_message_uuid').value.trim();
    if (!uuid) { $('msgRecallResFmt').textContent = '请输入消息UUID'; return; }
    recallMessage(uuid);
  };

  // Storage
  $('btnUploadFile').onclick = uploadFileWithHash;
  $('btnPreviewFile').onclick = previewFile;
  renderUploadedFiles();
}

async function register() {
  const body = {
    'user_id': $('reg_user_id').value.trim(),
    'nickname': $('reg_nickname').value.trim(),
    'password': $('reg_password').value,
  };
  const email = $('reg_email').value.trim();
  if (email) body['email'] = email;

  try {
    const res = await fetch(`${state.apiBase}/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const data = await res.json();
    $('regResult').textContent = pretty(data);
  } catch (err) {
    $('regResult').textContent = String(err);
  }
}

async function login() {
  const body = {
    'user_id': $('login_user_id').value.trim(),
    'password': $('login_password').value,
  };
  const device_info = $('login_device_info').value.trim();
  const mac = $('login_mac').value.trim();
  if (device_info) body['device_info'] = device_info;
  if (mac) body['mac_address'] = mac;

  try {
    const res = await fetch(`${state.apiBase}/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const data = await res.json();
    $('loginResult').textContent = pretty(data);
    if (res.ok && data.access_token && data.refresh_token) {
      // 使用TokenManager设置tokens
      await tokenManager.setTokens(data.access_token, data.refresh_token);
      renderLocalState();
      
      // 提示用户Service Worker状态
      if (!tokenManager.isServiceWorkerReady()) {
        $('loginResult').textContent += '\n\n⚠️ 首次登录需要刷新页面以启用流式播放\n点击刷新按钮或按F5';
      }
    }
  } catch (err) {
    $('loginResult').textContent = String(err);
  }
}

function decodeJwt(token) {
  try {
    const [, payload] = token.split('.');
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(json);
  } catch {
    return null;
  }
}

function claimsFromToken() {
  const c = decodeJwt(tokenManager.getAccessToken());
  return c || {};
}

function nowISO() { return new Date().toISOString(); }

function showRequest(id, req) {
  const { method, url, headers, body } = req;
  const out = { method, url, headers, body };
  $(id).textContent = pretty(out);
}

async function doJson(req, outId) {
  try {
    showRequest(outId, req);
    const res = await fetch(req.url, {
      method: req.method,
      headers: req.headers,
      body: req.body ? JSON.stringify(req.body) : undefined,
    });
    const data = await res.json().catch(() => ({}));
    return { ok: res.ok, data };
  } catch (err) {
    return { ok: false, data: { error: String(err) } };
  }
}

function showProfileFromAccessToken() {
  const accessToken = tokenManager.getAccessToken();
  if (!accessToken) {
    $('profile').textContent = '未登录或缺少 access_token';
    return;
  }
  const claims = decodeJwt(accessToken);
  if (!claims) {
    $('profile').textContent = '无法解析 Access Token';
    return;
  }
  const info = {
    user_id: claims.sub,
    email: claims.email,
    device_id: claims.device_id,
    device_info: claims.device_info,
    mac_address: claims.mac_address,
    exp: new Date(claims.exp * 1000).toISOString(),
    iat: new Date(claims.iat * 1000).toISOString(),
  };
  $('profile').textContent = pretty(info);
}

async function refreshAccessToken() {
  const refreshToken = tokenManager.getRefreshToken();
  if (!refreshToken) {
    $('profile').textContent = '缺少 refresh_token';
    return;
  }
  try {
    const res = await fetch(`${state.apiBase}/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
    const data = await res.json();
    if (res.ok && data.access_token) {
      // 使用TokenManager更新token
      await tokenManager.updateAccessToken(data.access_token);
      $('profile').textContent = '已刷新 Access Token';
      renderLocalState();
    } else {
      $('profile').textContent = pretty(data);
    }
  } catch (err) {
    $('profile').textContent = String(err);
  }
}

// Friends: list sent
async function listSentRequests() {
  if (!tokenManager.hasToken()) { $('sentResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}/requests/sent`,
    headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
  };
  showRequest('sentReqFmt', req);
  const { ok, data } = await doJson(req, 'sentReqFmt');
  $('sentResFmt').textContent = pretty(data);
  renderSentList(Array.isArray(data.items) ? data.items : []);
}

function renderSentList(items) {
  const root = $('sentList');
  root.innerHTML = '';
  for (const it of items) {
    const div = document.createElement('div');
    div.className = 'friend-item';
    div.innerHTML = `<strong>${it.sent_to_user_id}</strong> <span class="muted">${it.sent_time}</span><div class="muted">${it.sent_message || ''}</div>`;
    root.appendChild(div);
  }
}

// Friends: list pending
async function listPendingRequests() {
  if (!tokenManager.hasToken()) { $('pendingResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}/requests/pending`,
    headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
  };
  showRequest('pendingReqFmt', req);
  const { ok, data } = await doJson(req, 'pendingReqFmt');
  $('pendingResFmt').textContent = pretty(data);
  renderPendingList(Array.isArray(data.items) ? data.items : []);
}

function renderPendingList(items) {
  const root = $('pendingList');
  root.innerHTML = '';
  for (const it of items) {
    const row = document.createElement('div');
    row.className = 'friend-item';
    const info = document.createElement('div');
    info.innerHTML = `<strong>${it.request_user_id}</strong> <span class="muted">${it.request_time}</span><div class="muted">${it.request_message || ''}</div>`;
    const actions = document.createElement('div');

    const approveReason = document.createElement('input');
    approveReason.placeholder = '通过原因（可选）';
    approveReason.className = 'inline-input';
    const approveBtn = document.createElement('button');
    approveBtn.textContent = '同意';
    approveBtn.onclick = () => approveFriendRequest(it, approveReason.value.trim());

    const rejectReason = document.createElement('input');
    rejectReason.placeholder = '拒绝原因（可选）';
    rejectReason.className = 'inline-input';
    const rejectBtn = document.createElement('button');
    rejectBtn.textContent = '拒绝';
    rejectBtn.onclick = () => rejectFriendRequest(it, rejectReason.value.trim());

    actions.appendChild(approveReason);
    actions.appendChild(approveBtn);
    actions.appendChild(rejectReason);
    actions.appendChild(rejectBtn);

    row.appendChild(info);
    row.appendChild(actions);
    root.appendChild(row);
  }
}

// Friends: list owned
async function listFriends() {
  if (!tokenManager.hasToken()) { $('friendsResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}`,
    headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
  };
  showRequest('friendsReqFmt', req);
  const { ok, data } = await doJson(req, 'friendsReqFmt');
  $('friendsResFmt').textContent = pretty(data);
  renderFriends(Array.isArray(data.items) ? data.items : []);
}

function renderFriends(items) {
  const root = $('friendsList');
  root.innerHTML = '';
  for (const it of items) {
    const div = document.createElement('div');
    div.className = 'friend-item';
    const left = document.createElement('div');
    left.innerHTML = `<strong>${it.friend_id}</strong> <span class="muted">${it.add_time}</span><div class="muted">${it.approve_reason || ''}</div>`;
    const actions = document.createElement('div');
    const reasonInput = document.createElement('input');
    reasonInput.placeholder = '删除原因（可选）';
    reasonInput.className = 'inline-input';
    const btn = document.createElement('button');
    btn.textContent = '删除好友';
    btn.onclick = () => removeFriend(it.friend_id, reasonInput.value.trim());
    actions.appendChild(reasonInput);
    actions.appendChild(btn);
    div.appendChild(left);
    div.appendChild(actions);
    root.appendChild(div);
  }
}

// Friends: submit
async function submitFriendRequest() {
  if (!tokenManager.hasToken()) { $('submitResFmt').textContent = '未登录'; return; }
  const claims = claimsFromToken();
  const body = {
    user_id: claims.sub,
    target_user_id: $('req_target_user_id').value.trim(),
    request_time: $('req_request_time').value.trim() || nowISO(),
  };
  const reason = $('req_reason').value.trim();
  if (reason) body.reason = reason;

  const req = {
    method: 'POST',
    url: `${state.friendsBase}/requests`,
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    body,
  };
  showRequest('submitReqFmt', req);
  const { ok, data } = await doJson(req, 'submitReqFmt');
  $('submitResFmt').textContent = pretty(data);
  if (ok) { listSentRequests(); listPendingRequests(); }
}

async function removeFriend(friendId, reason) {
  if (!tokenManager.hasToken()) { $('removeResFmt').textContent = '未登录'; return; }
  const claims = claimsFromToken();
  const body = {
    user_id: claims.sub,
    friend_user_id: friendId,
    remove_time: nowISO(),
  };
  if (reason) body.remove_reason = reason;
  const req = {
    method: 'POST',
    url: `${state.friendsBase}/remove`,
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    body,
  };
  showRequest('removeReqFmt', req);
  const { ok, data } = await doJson(req, 'removeReqFmt');
  $('removeResFmt').textContent = pretty(data);
  if (ok) { listFriends(); }
}

// Friends: approve
async function approveFriendRequest(pendingItem, reason) {
  if (!tokenManager.hasToken()) { $('pendingResFmt').textContent = '未登录'; return; }
  const claims = claimsFromToken();
  const body = {
    user_id: claims.sub,
    applicant_user_id: pendingItem.request_user_id,
    approved_time: nowISO(),
  };
  if (reason) body.approved_reason = reason;
  const req = {
    method: 'POST',
    url: `${state.friendsBase}/requests/approve`,
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    body,
  };
  showRequest('pendingReqFmt', req);
  const { ok, data } = await doJson(req, 'pendingReqFmt');
  $('pendingResFmt').textContent = pretty(data);
  if (ok) { listPendingRequests(); listFriends(); }
}

// Friends: reject
async function rejectFriendRequest(pendingItem, reason) {
  if (!tokenManager.hasToken()) { $('pendingResFmt').textContent = '未登录'; return; }
  const claims = claimsFromToken();
  const body = {
    user_id: claims.sub,
    applicant_user_id: pendingItem.request_user_id,
  };
  if (reason) body.reject_reason = reason;
  const req = {
    method: 'POST',
    url: `${state.friendsBase}/requests/reject`,
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    body,
  };
  showRequest('pendingReqFmt', req);
  const { ok, data } = await doJson(req, 'pendingReqFmt');
  $('pendingResFmt').textContent = pretty(data);
  if (ok) { listPendingRequests(); }
}

async function listDevices() {
  if (!tokenManager.hasToken()) {
    $('deviceResult').textContent = '未登录或缺少 access_token';
    return;
  }
  try {
    const res = await fetch(`${state.apiBase}/devices`, {
      method: 'GET',
      headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    });
    const data = await res.json();
    $('deviceResult').textContent = pretty(data);
    if (res.ok && Array.isArray(data.devices)) {
      renderDevices(data.devices);
    }
  } catch (err) {
    $('deviceResult').textContent = String(err);
  }
}

function renderDevices(devices) {
  const root = $('devices');
  root.innerHTML = '';
  for (const d of devices) {
    const item = document.createElement('div');
    item.className = 'device-item';
    const left = document.createElement('div');
    left.innerHTML = `<strong>${d.device_id}</strong><div class="muted">${d.device_info || ''} | 最后活跃: ${d.last_used_at || ''}</div>`;
    const del = document.createElement('button');
    del.textContent = '删除设备';
    del.onclick = () => deleteDevice(d.device_id);
    item.appendChild(left);
    item.appendChild(del);
    root.appendChild(item);
  }
}

async function deleteDevice(deviceId) {
  if (!tokenManager.hasToken()) return;
  try {
    const res = await fetch(`${state.apiBase}/devices/${encodeURIComponent(deviceId)}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
    });
    const data = await res.json();
    $('deviceResult').textContent = pretty(data);
    if (res.ok) {
      await listDevices();
    }
  } catch (err) {
    $('deviceResult').textContent = String(err);
  }
}

function renderLocalState() {
  $('localState').textContent = pretty({
    apiBase: state.apiBase,
    friendsApiBase: state.friendsBase,
    messagesApiBase: state.messagesBase,
    hasAccessToken: tokenManager.hasToken(),
    hasRefreshToken: !!tokenManager.getRefreshToken(),
    serviceWorkerReady: tokenManager.isServiceWorkerReady(),
  });
}

// ========================================
// Messages API
// ========================================

// Send message
async function sendMessage() {
  if (!tokenManager.hasToken()) { $('msgSendResFmt').textContent = '未登录'; return; }
  
  const body = {
    receiver_id: $('msg_receiver_id').value.trim(),
    message_content: $('msg_content').value.trim(),
    message_type: $('msg_type').value || 'text',
  };
  
  const fileUrl = $('msg_file_url').value.trim();
  const fileSize = $('msg_file_size').value.trim();
  if (fileUrl) body.file_url = fileUrl;
  if (fileSize) body.file_size = parseInt(fileSize);

  if (!body.receiver_id || !body.message_content) {
    $('msgSendResFmt').textContent = '接收者ID和消息内容必填';
    return;
  }

  const req = {
    method: 'POST',
    url: state.messagesBase,
    headers: { 
      'Content-Type': 'application/json', 
      'Authorization': `Bearer ${tokenManager.getAccessToken()}` 
    },
    body,
  };
  showRequest('msgSendReqFmt', req);
  const { ok, data } = await doJson(req, 'msgSendReqFmt');
  $('msgSendResFmt').textContent = pretty(data);
  
  // 发送成功后清空表单
  if (ok) {
    $('msg_content').value = '';
    $('msg_file_url').value = '';
    $('msg_file_size').value = '';
    // 如果正在查看与该好友的消息，自动刷新
    const currentFriendId = $('msg_friend_id').value.trim();
    if (currentFriendId === body.receiver_id) {
      setTimeout(() => getMessages(), 500);
    }
  }
}

// Get messages
async function getMessages() {
  if (!tokenManager.hasToken()) { $('msgGetResFmt').textContent = '未登录'; return; }
  
  const friendId = $('msg_friend_id').value.trim();
  const beforeUuid = $('msg_before_uuid').value.trim();
  const limit = $('msg_limit').value.trim() || '50';

  if (!friendId) {
    $('msgGetResFmt').textContent = '好友ID必填';
    return;
  }

  let url = `${state.messagesBase}?friend_id=${encodeURIComponent(friendId)}&limit=${limit}`;
  if (beforeUuid) {
    url += `&before_uuid=${encodeURIComponent(beforeUuid)}`;
  }

  const req = {
    method: 'GET',
    url,
    headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` },
  };
  showRequest('msgGetReqFmt', req);
  const { ok, data } = await doJson(req, 'msgGetReqFmt');
  $('msgGetResFmt').textContent = pretty(data);
  
  // 渲染消息列表
  if (ok && Array.isArray(data.messages)) {
    renderMessages(data.messages, data.has_more);
  }
}

function renderMessages(messages, hasMore) {
  const root = $('messagesList');
  root.innerHTML = '';
  
  if (messages.length === 0) {
    root.innerHTML = '<div class="muted">暂无消息</div>';
    return;
  }

  const claims = claimsFromToken();
  const currentUserId = claims.sub;

  for (const msg of messages) {
    const item = document.createElement('div');
    item.className = 'message-item';
    if (msg.sender_id === currentUserId) {
      item.classList.add('sent');
    } else {
      item.classList.add('received');
    }

    const header = document.createElement('div');
    header.className = 'message-header';
    header.innerHTML = `
      <strong>${msg.sender_id === currentUserId ? '我' : msg.sender_id}</strong>
      <span class="muted">${new Date(msg.send_time).toLocaleString()}</span>
    `;

    const content = document.createElement('div');
    content.className = 'message-content';
    
    if (msg.message_type === 'text') {
      content.textContent = msg.message_content;
    } else {
      content.innerHTML = `
        <div><strong>[${msg.message_type}]</strong> ${msg.message_content || ''}</div>
        ${msg.file_url ? `<div class="muted">文件: ${msg.file_url}</div>` : ''}
        ${msg.file_size ? `<div class="muted">大小: ${(msg.file_size / 1024).toFixed(2)} KB</div>` : ''}
      `;
    }

    const footer = document.createElement('div');
    footer.className = 'message-footer';
    footer.innerHTML = `<span class="muted">UUID: ${msg.message_uuid}</span>`;

    const actions = document.createElement('div');
    actions.className = 'message-actions';
    
    // 删除按钮
    const deleteBtn = document.createElement('button');
    deleteBtn.textContent = '删除';
    deleteBtn.className = 'small-btn';
    deleteBtn.onclick = () => {
      $('delete_message_uuid').value = msg.message_uuid;
      deleteMessage(msg.message_uuid);
    };
    actions.appendChild(deleteBtn);

    // 撤回按钮（只有发送者且2分钟内）
    if (msg.sender_id === currentUserId) {
      const sendTime = new Date(msg.send_time);
      const now = new Date();
      const diffMinutes = (now - sendTime) / 1000 / 60;
      
      if (diffMinutes <= 2) {
        const recallBtn = document.createElement('button');
        recallBtn.textContent = '撤回';
        recallBtn.className = 'small-btn';
        recallBtn.onclick = () => {
          $('recall_message_uuid').value = msg.message_uuid;
          recallMessage(msg.message_uuid);
        };
        actions.appendChild(recallBtn);
      }
    }

    item.appendChild(header);
    item.appendChild(content);
    item.appendChild(footer);
    item.appendChild(actions);
    root.appendChild(item);
  }

  // 显示是否还有更多消息
  if (hasMore) {
    const moreDiv = document.createElement('div');
    moreDiv.className = 'muted';
    moreDiv.style.textAlign = 'center';
    moreDiv.style.marginTop = '10px';
    moreDiv.innerHTML = '还有更多消息...';
    
    const loadMoreBtn = document.createElement('button');
    loadMoreBtn.textContent = '加载更多';
    loadMoreBtn.onclick = () => {
      const lastMsg = messages[messages.length - 1];
      $('msg_before_uuid').value = lastMsg.message_uuid;
      getMessages();
    };
    moreDiv.appendChild(loadMoreBtn);
    root.appendChild(moreDiv);
  }
}

// Delete message
async function deleteMessage(uuid) {
  if (!tokenManager.hasToken()) { $('msgDeleteResFmt').textContent = '未登录'; return; }
  if (!uuid) { $('msgDeleteResFmt').textContent = '消息UUID必填'; return; }

  const req = {
    method: 'DELETE',
    url: `${state.messagesBase}/delete`,
    headers: { 
      'Content-Type': 'application/json', 
      'Authorization': `Bearer ${tokenManager.getAccessToken()}` 
    },
    body: { message_uuid: uuid },
  };
  showRequest('msgDeleteReqFmt', req);
  const { ok, data } = await doJson(req, 'msgDeleteReqFmt');
  $('msgDeleteResFmt').textContent = pretty(data);
  
  // 删除成功后刷新消息列表
  if (ok) {
    $('delete_message_uuid').value = '';
    const friendId = $('msg_friend_id').value.trim();
    if (friendId) {
      setTimeout(() => getMessages(), 500);
    }
  }
}

// Recall message
async function recallMessage(uuid) {
  if (!tokenManager.hasToken()) { $('msgRecallResFmt').textContent = '未登录'; return; }
  if (!uuid) { $('msgRecallResFmt').textContent = '消息UUID必填'; return; }

  const req = {
    method: 'POST',
    url: `${state.messagesBase}/recall`,
    headers: { 
      'Content-Type': 'application/json', 
      'Authorization': `Bearer ${tokenManager.getAccessToken()}` 
    },
    body: { message_uuid: uuid },
  };
  showRequest('msgRecallReqFmt', req);
  const { ok, data } = await doJson(req, 'msgRecallReqFmt');
  $('msgRecallResFmt').textContent = pretty(data);
  
  // 撤回成功后刷新消息列表
  if (ok) {
    $('recall_message_uuid').value = '';
    const friendId = $('msg_friend_id').value.trim();
    if (friendId) {
      setTimeout(() => getMessages(), 500);
    }
  }
}

// ==================== Storage Functions ====================

// 计算文件采样SHA-256哈希（适用于所有文件大小）
// 采样策略：文件元信息 + 开头10MB + 中间10MB + 结尾10MB
async function calculateSHA256(file) {
  try {
    const SAMPLE_SIZE = 10 * 1024 * 1024; // 10MB
    
    // 文件元信息
    const metadata = `${file.name}|${file.size}|${file.lastModified}|${file.type}`;
    const metadataBuffer = new TextEncoder().encode(metadata);
    
    let dataToHash;
    
    if (file.size <= SAMPLE_SIZE * 3) {
      // 小文件（< 30MB）：计算完整哈希
      dataToHash = await file.arrayBuffer();
    } else {
      // 大文件：采样哈希策略
      const chunks = [];
      
      // 1. 读取开头10MB
      const startBlob = file.slice(0, SAMPLE_SIZE);
      const startBuffer = await startBlob.arrayBuffer();
      chunks.push(new Uint8Array(startBuffer));
      
      // 2. 读取中间10MB
      const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
      const middleBlob = file.slice(middleStart, middleStart + SAMPLE_SIZE);
      const middleBuffer = await middleBlob.arrayBuffer();
      chunks.push(new Uint8Array(middleBuffer));
      
      // 3. 读取结尾10MB
      const endBlob = file.slice(file.size - SAMPLE_SIZE, file.size);
      const endBuffer = await endBlob.arrayBuffer();
      chunks.push(new Uint8Array(endBuffer));
      
      // 合并所有采样数据
      const totalLength = metadataBuffer.length + chunks.reduce((sum, chunk) => sum + chunk.length, 0);
      dataToHash = new Uint8Array(totalLength);
      let offset = 0;
      
      // 添加元信息
      dataToHash.set(metadataBuffer, offset);
      offset += metadataBuffer.length;
      
      // 添加采样数据
      for (const chunk of chunks) {
        dataToHash.set(chunk, offset);
        offset += chunk.length;
      }
    }
    
    // 计算SHA-256哈希
    const hashBuffer = await crypto.subtle.digest('SHA-256', dataToHash);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  } catch (error) {
    throw new Error('计算哈希失败: ' + error.message);
  }
}

// 更新进度条
function updateProgress(percent, text) {
  $('uploadProgress').style.display = 'block';
  $('progressBar').style.width = percent + '%';
  $('progressText').textContent = text;
}

// 完整的文件上传流程
async function uploadFileWithHash() {
  const fileInput = $('file_input');
  const file = fileInput.files[0];
  
  if (!file) {
    $('fileUploadResFmt').textContent = '请选择文件';
    return;
  }

  if (!tokenManager.hasToken()) {
    $('fileUploadResFmt').textContent = '未登录，请先登录';
    return;
  }

  try {
    updateProgress(0, '准备上传...');
    
    // 1. 计算文件采样哈希
    updateProgress(10, file.size > 30 * 1024 * 1024 
      ? '正在计算采样哈希（开头+中间+结尾）...' 
      : '正在计算完整哈希...');
    const fileHash = await calculateSHA256(file);
    console.log('文件哈希:', fileHash);
    
    // 2. 准备请求数据
    const fileType = $('file_type').value;
    const storageLocation = $('storage_location').value;
    const relatedId = $('related_id').value.trim() || null;
    const forceUpload = $('force_upload').checked;
    
    const requestBody = {
      file_type: fileType,
      storage_location: storageLocation,
      related_id: relatedId,
      filename: file.name,
      file_size: file.size,
      content_type: file.type || 'application/octet-stream',
      file_hash: fileHash,
      force_upload: forceUpload
    };

    // 超大文件需要用户指定时间
    if (file.size > 15 * 1024 * 1024 * 1024) {
      const hours = prompt('文件超过15GB，请输入预计上传时间（小时）:', '24');
      if (!hours) {
        updateProgress(0, '已取消');
        return;
      }
      requestBody.estimated_upload_time = parseInt(hours) * 3600;
    }

    // 3. 请求上传
    updateProgress(20, '请求上传URL...');
    const req = {
      method: 'POST',
      url: `${state.storageBase}/upload/request`,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${tokenManager.getAccessToken()}`
      },
      body: requestBody
    };

    showRequest('fileUploadReqFmt', req);
    const { ok, data: uploadInfo } = await doJson(req, 'fileUploadReqFmt');
    
    if (!ok) {
      $('fileUploadResFmt').textContent = pretty(uploadInfo);
      updateProgress(0, '请求失败');
      return;
    }

    $('fileUploadResFmt').textContent = pretty(uploadInfo);

    // 4. 检查秒传
    if (uploadInfo.instant_upload) {
      updateProgress(100, '秒传成功！');
      console.log('秒传成功:', uploadInfo.existing_file_url);
      
      // 保存到已上传列表
      addUploadedFile({
        filename: file.name,
        file_url: uploadInfo.existing_file_url,
        file_key: uploadInfo.file_key,
        instant: true,
        timestamp: new Date().toISOString()
      });
      
      setTimeout(() => {
        updateProgress(0, '');
        $('uploadProgress').style.display = 'none';
      }, 2000);
      return;
    }

    // 5. 根据模式上传
    if (uploadInfo.mode === 'one_time_token') {
      // 小文件直接上传
      updateProgress(30, '正在上传文件到MinIO...');
      await uploadFileWithToken(file, uploadInfo);
    } else {
      // 超大文件分片上传
      updateProgress(30, '开始分片上传...');
      await uploadLargeFileMultipart(file, uploadInfo);
    }

  } catch (error) {
    console.error('上传失败:', error);
    $('fileUploadResFmt').textContent = '上传失败: ' + error.message;
    updateProgress(0, '上传失败');
  }
}

// 使用一次性Token上传
async function uploadFileWithToken(file, uploadInfo) {
  const formData = new FormData();
  formData.append('file', file);

  try {
    const response = await fetch(uploadInfo.upload_url, {
      method: 'POST',
      body: formData
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || '上传失败');
    }

    const result = await response.json();
    updateProgress(100, '上传成功！');
    
    console.log('上传成功:', result);
    $('fileUploadResFmt').textContent = pretty(result);

    // 保存到已上传列表
    addUploadedFile({
      filename: file.name,
      file_url: result.file_url,
      file_key: result.file_key,
      file_size: result.file_size,
      preview_support: result.preview_support,
      instant: false,
      timestamp: new Date().toISOString()
    });

    setTimeout(() => {
      updateProgress(0, '');
      $('uploadProgress').style.display = 'none';
      $('file_input').value = '';
    }, 2000);

  } catch (error) {
    throw new Error('上传到MinIO失败: ' + error.message);
  }
}

// 超大文件分片上传
async function uploadLargeFileMultipart(file, uploadInfo) {
  const chunkSize = 50 * 1024 * 1024; // 50MB
  const chunks = Math.ceil(file.size / chunkSize);
  
  for (let i = 0; i < chunks; i++) {
    const start = i * chunkSize;
    const end = Math.min(start + chunkSize, file.size);
    const chunk = file.slice(start, end);
    
    const progress = 30 + Math.floor((i / chunks) * 60);
    updateProgress(progress, `上传分片 ${i + 1}/${chunks}...`);
    
    // 获取分片URL
    const partUrlResponse = await fetch(
      `${state.storageBase}/multipart/part-url?file_key=${uploadInfo.file_key}&upload_id=${uploadInfo.multipart_upload_id}&part_number=${i + 1}`,
      {
        headers: {
          'Authorization': `Bearer ${tokenManager.getAccessToken()}`
        }
      }
    );
    
    if (!partUrlResponse.ok) {
      throw new Error('获取分片URL失败');
    }
    
    const { part_url } = await partUrlResponse.json();
    
    // 上传分片
    const uploadResponse = await fetch(part_url, {
      method: 'PUT',
      body: chunk
    });
    
    if (!uploadResponse.ok) {
      throw new Error(`分片 ${i + 1} 上传失败`);
    }
    
    console.log(`分片 ${i + 1}/${chunks} 上传完成`);
  }
  
  updateProgress(100, '分片上传完成！');
  
  setTimeout(() => {
    updateProgress(0, '');
    $('uploadProgress').style.display = 'none';
    $('file_input').value = '';
  }, 2000);
}

// 添加到已上传列表
function addUploadedFile(fileInfo) {
  state.uploadedFiles.unshift(fileInfo);
  if (state.uploadedFiles.length > 10) {
    state.uploadedFiles = state.uploadedFiles.slice(0, 10);
  }
  localStorage.setItem('uploadedFiles', JSON.stringify(state.uploadedFiles));
  renderUploadedFiles();
}

// 渲染已上传文件列表
function renderUploadedFiles() {
  const container = $('uploadedFilesList');
  if (!container) return;
  
  if (state.uploadedFiles.length === 0) {
    container.innerHTML = '<p style="color: #999;">暂无上传文件</p>';
    return;
  }
  
  container.innerHTML = state.uploadedFiles.map((file, index) => `
    <div style="padding: 10px; margin: 5px 0; background: ${file.instant ? '#e8f5e9' : '#f5f5f5'}; border-radius: 5px;">
      <div style="font-weight: bold;">${file.filename} ${file.instant ? '⚡秒传' : ''}</div>
      <div style="font-size: 0.9em; color: #666; margin: 5px 0;">
        文件key: ${file.file_key}
      </div>
      ${file.file_size ? `<div style="font-size: 0.9em;">大小: ${formatFileSize(file.file_size)}</div>` : ''}
      <div style="font-size: 0.9em;">
        时间: ${new Date(file.timestamp).toLocaleString()}
      </div>
      <div style="margin-top: 5px;">
        <a href="${file.file_url}" target="_blank" style="color: #007bff; text-decoration: none;">
          ${file.preview_support === 'inline_preview' ? '🖼️ 预览' : '📥 下载'}
        </a>
        <button onclick="previewUploadedFile('${file.file_url}', '${file.filename}')" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em;">📺 在线查看</button>
        <button onclick="copyToClipboard('${file.file_url}')" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em;">复制URL</button>
        <button onclick="removeUploadedFile(${index})" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em; background: #f44336; color: white;">删除记录</button>
      </div>
    </div>
  `).join('');
}

// 格式化文件大小
function formatFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(2) + ' MB';
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}

// 复制到剪贴板
function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    alert('已复制到剪贴板');
  }).catch(() => {
    alert('复制失败');
  });
}

// 删除上传记录
function removeUploadedFile(index) {
  state.uploadedFiles.splice(index, 1);
  localStorage.setItem('uploadedFiles', JSON.stringify(state.uploadedFiles));
  renderUploadedFiles();
}

// =============================================
// 文件预览功能
// =============================================

// 预览文件（根据文件名后缀判断类型）
async function previewFile(filename = '') {
  const urlInput = document.getElementById('preview_url');
  const previewContainer = document.getElementById('filePreview');
  const url = urlInput.value.trim();
  
  if (!url) {
    previewContainer.innerHTML = '<p style="color: #999;">请输入文件URL</p>';
    return;
  }
  
  // 判断是否是UUID格式的URL（需要Authorization）
  const isUuidUrl = url.includes('/api/storage/file/');
  
  // 确定用于判断类型的字符串（优先使用filename，否则使用URL）
  const checkString = filename || url;
  const lowerStr = checkString.toLowerCase();
  
  // 根据文件名后缀判断文件类型
  if (lowerStr.match(/\.(jpg|jpeg|png|gif|webp)$/i)) {
    // 图片预览
    if (isUuidUrl) {
      await previewImageUuid(url, previewContainer, filename);
    } else {
      previewContainer.innerHTML = `
        <div>
          <h4 style="margin-bottom: 10px;">图片预览</h4>
          <img src="${url}" alt="图片预览" style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);" 
               onerror="this.parentElement.innerHTML='<p style=color:red>图片加载失败</p>'" />
          <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
            <a href="${url}" target="_blank" style="color: #007bff;">在新标签页打开</a> | 
            <a href="${url}" download style="color: #007bff;">下载图片</a>
          </div>
        </div>
      `;
    }
  } else if (lowerStr.match(/\.(mp4|webm|ogg|mov)$/i)) {
    // 视频预览
    if (isUuidUrl) {
      await previewVideoUuid(url, previewContainer, filename);
    } else {
      previewContainer.innerHTML = `
        <div>
          <h4 style="margin-bottom: 10px;">视频预览</h4>
          <video controls style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);" 
                 onerror="this.parentElement.innerHTML='<p style=color:red>视频加载失败</p>'">
            <source src="${url}" type="video/mp4">
            您的浏览器不支持视频播放
          </video>
          <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
            <a href="${url}" target="_blank" style="color: #007bff;">在新标签页打开</a> | 
            <a href="${url}" download style="color: #007bff;">下载视频</a>
          </div>
        </div>
      `;
    }
  } else if (lowerStr.match(/\.pdf$/i)) {
    // PDF预览
    if (isUuidUrl) {
      await previewPdfUuid(url, previewContainer, filename);
    } else {
      previewContainer.innerHTML = `
        <div>
          <h4 style="margin-bottom: 10px;">PDF预览</h4>
          <iframe src="${url}" style="width: 100%; height: 600px; border: none; border-radius: 5px;" 
                  onerror="this.parentElement.innerHTML='<p style=color:red>PDF加载失败</p>'"></iframe>
          <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
            <a href="${url}" target="_blank" style="color: #007bff;">在新标签页打开</a> | 
            <a href="${url}" download style="color: #007bff;">下载PDF</a>
          </div>
        </div>
      `;
    }
  } else {
    // 其他文件类型
    if (isUuidUrl) {
      previewContainer.innerHTML = `
        <div>
          <h4 style="margin-bottom: 10px;">文件信息 (UUID访问)</h4>
          <p>文件名: ${filename || '未知'}</p>
          <p style="color: #666; word-break: break-all;">UUID: ${url.split('/').pop()}</p>
          <p style="color: #4caf50;">🔗 需要权限访问</p>
          <div style="margin-top: 15px;">
            <button onclick="downloadUuidFile('${url}', '${filename || url.split('/').pop()}')" style="padding: 10px 20px; background: #28a745; color: white; border: none; border-radius: 5px; cursor: pointer;">📥 下载文件</button>
          </div>
        </div>
      `;
    } else {
      previewContainer.innerHTML = `
        <div>
          <h4 style="margin-bottom: 10px;">文件信息</h4>
          <p>该文件类型不支持在线预览</p>
          <p style="color: #666; word-break: break-all;">${url}</p>
          <div style="margin-top: 15px;">
            <a href="${url}" target="_blank" style="display: inline-block; padding: 10px 20px; background: #007bff; color: white; text-decoration: none; border-radius: 5px;">在新标签页打开</a>
            <a href="${url}" download style="display: inline-block; margin-left: 10px; padding: 10px 20px; background: #28a745; color: white; text-decoration: none; border-radius: 5px;">下载文件</a>
          </div>
        </div>
      `;
    }
  }
}

// UUID图片预览
async function previewImageUuid(url, container, filename) {
  container.innerHTML = '<p>正在加载图片...</p>';
  try {
    const response = await fetch(url, {
      headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` }
    });
    if (!response.ok) throw new Error('无权访问或文件不存在');
    const blob = await response.blob();
    const blobUrl = URL.createObjectURL(blob);
    container.innerHTML = `
      <div>
        <h4 style="margin-bottom: 10px;">图片预览 (UUID访问)</h4>
        <img src="${blobUrl}" alt="图片预览" style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);" />
        <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
          ${filename ? `文件名: ${filename}<br/>` : ''}
          UUID: ${url.split('/').pop()}
        </div>
        <div style="margin-top: 10px;">
          <button onclick="downloadUuidFile('${url}', '${filename || url.split('/').pop() + '.jpg'}')" style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 5px; cursor: pointer;">📥 下载图片</button>
        </div>
      </div>
    `;
  } catch (error) {
    container.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
  }
}

// UUID视频预览（通过Service Worker实现真正的流式播放）
async function previewVideoUuid(url, container, filename) {
  // 检查是否已登录
  if (!tokenManager.hasToken()) {
    container.innerHTML = `
      <div>
        <h4 style="margin-bottom: 10px;">⚠️ 未登录</h4>
        <p style="color: #ff9800;">请先登录以访问文件</p>
      </div>
    `;
    return;
  }
  
  // 检查Service Worker状态
  if (!tokenManager.isServiceWorkerReady()) {
    container.innerHTML = `
      <div>
        <h4 style="margin-bottom: 10px;">⚠️ Service Worker 未就绪</h4>
        <p style="color: #ff9800;">首次使用需要刷新页面以启用流式播放支持</p>
        <button onclick="location.reload()" style="margin-top: 10px; padding: 8px 16px; background: #2196f3; color: white; border: none; border-radius: 5px; cursor: pointer;">刷新页面</button>
        <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
          <button onclick="tokenManager.diagnose()" style="padding: 6px 12px; font-size: 0.9em;">🔍 运行诊断</button>
        </div>
      </div>
    `;
    return;
  }
  
  container.innerHTML = `
    <div>
      <h4 style="margin-bottom: 10px;">视频预览 (UUID访问 - 真正流式播放 🚀)</h4>
      <video 
        id="uuidVideo" 
        controls 
        preload="metadata"
        style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);">
        <source src="${url}" type="video/mp4">
        您的浏览器不支持视频播放
      </video>
      <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
        ${filename ? `📁 文件名: ${filename}<br/>` : ''}
        🔗 UUID: ${url.split('/').pop()}<br/>
        <span id="videoStatus">
          ✅ 支持流式播放：
          <ul style="margin: 5px 0; padding-left: 20px;">
            <li>边看边加载，无需等待完整下载</li>
            <li>支持拖动进度条立即响应</li>
            <li>支持快进/快退</li>
            <li>节省流量和内存</li>
          </ul>
        </span>
      </div>
      <div style="margin-top: 10px;">
        <button onclick="downloadUuidFile('${url}', '${filename || url.split('/').pop() + '.mp4'}')" style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 5px; cursor: pointer;">📥 下载视频</button>
      </div>
    </div>
  `;
  
  const video = document.getElementById('uuidVideo');
  const statusSpan = document.getElementById('videoStatus');
  
  // 监听视频事件
  video.addEventListener('loadstart', () => {
    console.log('[Video] 开始加载');
  });
  
  video.addEventListener('loadedmetadata', () => {
    console.log('[Video] 元数据加载完成');
    const duration = video.duration;
    const minutes = Math.floor(duration / 60);
    const seconds = Math.floor(duration % 60);
    statusSpan.innerHTML += `<br/>⏱️ 视频时长: ${minutes}:${seconds.toString().padStart(2, '0')}`;
  });
  
  video.addEventListener('progress', () => {
    if (video.buffered.length > 0) {
      const buffered = video.buffered.end(0);
      const duration = video.duration;
      const percent = (buffered / duration * 100).toFixed(1);
      console.log(`[Video] 缓冲进度: ${percent}%`);
    }
  });
  
  video.addEventListener('canplay', () => {
    console.log('[Video] 可以开始播放');
  });
  
  video.addEventListener('error', (e) => {
    console.error('[Video] 播放错误:', e);
    const error = video.error;
    let errorMsg = '视频加载失败';
    let troubleshoot = '';
    
    if (error) {
      switch(error.code) {
        case error.MEDIA_ERR_ABORTED:
          errorMsg = '播放被中止';
          break;
        case error.MEDIA_ERR_NETWORK:
          errorMsg = '网络错误';
          troubleshoot = '请检查网络连接';
          break;
        case error.MEDIA_ERR_DECODE:
          errorMsg = '解码错误';
          troubleshoot = '视频格式可能不受支持';
          break;
        case error.MEDIA_ERR_SRC_NOT_SUPPORTED:
          errorMsg = '无权访问或不支持的视频格式';
          troubleshoot = `
            <br/><br/>可能的原因：
            <ul style="text-align: left; margin: 10px 0;">
              <li>❌ 未登录或token已过期 - 请重新登录</li>
              <li>❌ Service Worker未同步token - 请刷新页面</li>
              <li>❌ 没有访问权限 - 请检查文件权限</li>
            </ul>
            <button onclick="location.reload()" style="margin-top: 10px; padding: 8px 16px; background: #2196f3; color: white; border: none; border-radius: 5px; cursor: pointer;">刷新页面重试</button>
          `;
          break;
      }
    }
    
    container.innerHTML = `
      <div style="text-align: center; padding: 20px;">
        <p style="color: red; font-size: 1.2em;">❌ ${errorMsg}</p>
        ${troubleshoot}
      </div>
    `;
  });
}

// UUID PDF预览
async function previewPdfUuid(url, container, filename) {
  container.innerHTML = '<p>正在加载PDF...</p>';
  try {
    const response = await fetch(url, {
      headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` }
    });
    if (!response.ok) throw new Error('无权访问或文件不存在');
    const blob = await response.blob();
    const blobUrl = URL.createObjectURL(blob);
    container.innerHTML = `
      <div>
        <h4 style="margin-bottom: 10px;">PDF预览 (UUID访问)</h4>
        <iframe src="${blobUrl}" style="width: 100%; height: 600px; border: none; border-radius: 5px;"></iframe>
        <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
          ${filename ? `文件名: ${filename}<br/>` : ''}
          UUID: ${url.split('/').pop()}
        </div>
        <div style="margin-top: 10px;">
          <button onclick="downloadUuidFile('${url}', '${filename || url.split('/').pop() + '.pdf'}')" style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 5px; cursor: pointer;">📥 下载PDF</button>
        </div>
      </div>
    `;
  } catch (error) {
    container.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
  }
}

// 从已上传文件列表预览
function previewUploadedFile(url, filename) {
  document.getElementById('preview_url').value = url;
  previewFile(filename);
  // 滚动到预览区域
  document.getElementById('filePreview').scrollIntoView({ behavior: 'smooth', block: 'center' });
}

// =============================================
// UUID 文件下载功能（需要权限验证）
// =============================================
async function downloadUuidFile(url, filename) {
  if (!tokenManager.hasToken()) {
    alert('未登录，请先登录');
    return;
  }
  
  try {
    // 显示下载提示
    console.log('开始下载文件:', filename);
    
    // 获取文件
    const response = await fetch(url, {
      headers: { 'Authorization': `Bearer ${tokenManager.getAccessToken()}` }
    });
    
    if (!response.ok) {
      throw new Error('无权访问或文件不存在');
    }
    
    // 获取文件blob
    const blob = await response.blob();
    
    // 从响应头获取文件名（如果有）
    const contentDisposition = response.headers.get('content-disposition');
    let downloadFilename = filename;
    if (contentDisposition) {
      const filenameMatch = contentDisposition.match(/filename[^;=\n]*=((['"]).*?\2|[^;\n]*)/);
      if (filenameMatch && filenameMatch[1]) {
        downloadFilename = filenameMatch[1].replace(/['"]/g, '');
      }
    }
    
    // 创建下载链接
    const blobUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = blobUrl;
    a.download = downloadFilename || 'download';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    // 释放blob URL
    setTimeout(() => URL.revokeObjectURL(blobUrl), 100);
    
    console.log('文件下载成功:', downloadFilename);
    
  } catch (error) {
    alert('下载失败: ' + error.message);
    console.error('下载失败:', error);
  }
}

window.addEventListener('DOMContentLoaded', init);