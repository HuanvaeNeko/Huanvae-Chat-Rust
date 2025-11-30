const $ = (id) => document.getElementById(id);
const pretty = (obj) => JSON.stringify(obj, null, 2);

const state = {
  apiBase: localStorage.getItem('apiBase') || 'http://localhost:8080/api/auth',
  friendsBase: localStorage.getItem('friendsApiBase') || 'http://localhost:8080/api/friends',
  messagesBase: localStorage.getItem('messagesApiBase') || 'http://localhost:8080/api/messages',
  storageBase: localStorage.getItem('storageApiBase') || 'http://localhost:8080/api/storage',
  accessToken: localStorage.getItem('accessToken') || '',
  refreshToken: localStorage.getItem('refreshToken') || '',
  uploadedFiles: JSON.parse(localStorage.getItem('uploadedFiles') || '[]'),
  presignedUrls: {}, // 缓存预签名URL: { uuid: { url, expiresAt } }
};

function init() {
  $('apiBase').value = state.apiBase;
  $('friendsApiBase').value = state.friendsBase;
  $('messagesApiBase').value = state.messagesBase;
  $('storageApiBase').value = state.storageBase;
  renderLocalState();
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
    state.accessToken = '';
    state.refreshToken = '';
    localStorage.removeItem('accessToken');
    localStorage.removeItem('refreshToken');
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
      state.accessToken = data.access_token;
      state.refreshToken = data.refresh_token;
      localStorage.setItem('accessToken', state.accessToken);
      localStorage.setItem('refreshToken', state.refreshToken);
      renderLocalState();
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
  const c = decodeJwt(state.accessToken);
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
  if (!state.accessToken) {
    $('profile').textContent = '未登录或缺少 access_token';
    return;
  }
  const claims = decodeJwt(state.accessToken);
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
  if (!state.refreshToken) {
    const error = '缺少 refresh_token';
    if ($('profile')) $('profile').textContent = error;
    throw new Error(error);
  }
  
  console.log('🔄 正在刷新Access Token...');
  
  try {
    const res = await fetch(`${state.apiBase}/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: state.refreshToken }),
    });
    
    const data = await res.json();
    
    if (res.ok && data.access_token) {
      // 更新内存和localStorage
      state.accessToken = data.access_token;
      localStorage.setItem('accessToken', state.accessToken);
      
      if ($('profile')) $('profile').textContent = '✅ Access Token已刷新';
      renderLocalState();
      
      console.log('✅ Access Token刷新成功');
      console.log('新Token过期时间:', new Date(decodeJwt(state.accessToken)?.exp * 1000).toISOString());
    } else {
      const error = data.error || 'Token刷新失败';
      console.error('❌ Token刷新失败:', error);
      if ($('profile')) $('profile').textContent = pretty(data);
      throw new Error(error);
    }
  } catch (err) {
    const error = String(err);
    console.error('❌ Token刷新异常:', error);
    if ($('profile')) $('profile').textContent = error;
    throw err;
  }
}

// Friends: list sent
async function listSentRequests() {
  if (!state.accessToken) { $('sentResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}/requests/sent`,
    headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
  if (!state.accessToken) { $('pendingResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}/requests/pending`,
    headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
  if (!state.accessToken) { $('friendsResFmt').textContent = '未登录'; return; }
  const req = {
    method: 'GET',
    url: `${state.friendsBase}`,
    headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
  if (!state.accessToken) { $('submitResFmt').textContent = '未登录'; return; }
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
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${state.accessToken}` },
    body,
  };
  showRequest('submitReqFmt', req);
  const { ok, data } = await doJson(req, 'submitReqFmt');
  $('submitResFmt').textContent = pretty(data);
  if (ok) { listSentRequests(); listPendingRequests(); }
}

async function removeFriend(friendId, reason) {
  if (!state.accessToken) { $('removeResFmt').textContent = '未登录'; return; }
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
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${state.accessToken}` },
    body,
  };
  showRequest('removeReqFmt', req);
  const { ok, data } = await doJson(req, 'removeReqFmt');
  $('removeResFmt').textContent = pretty(data);
  if (ok) { listFriends(); }
}

// Friends: approve
async function approveFriendRequest(pendingItem, reason) {
  if (!state.accessToken) { $('pendingResFmt').textContent = '未登录'; return; }
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
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${state.accessToken}` },
    body,
  };
  showRequest('pendingReqFmt', req);
  const { ok, data } = await doJson(req, 'pendingReqFmt');
  $('pendingResFmt').textContent = pretty(data);
  if (ok) { listPendingRequests(); listFriends(); }
}

// Friends: reject
async function rejectFriendRequest(pendingItem, reason) {
  if (!state.accessToken) { $('pendingResFmt').textContent = '未登录'; return; }
  const claims = claimsFromToken();
  const body = {
    user_id: claims.sub,
    applicant_user_id: pendingItem.request_user_id,
  };
  if (reason) body.reject_reason = reason;
  const req = {
    method: 'POST',
    url: `${state.friendsBase}/requests/reject`,
    headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${state.accessToken}` },
    body,
  };
  showRequest('pendingReqFmt', req);
  const { ok, data } = await doJson(req, 'pendingReqFmt');
  $('pendingResFmt').textContent = pretty(data);
  if (ok) { listPendingRequests(); }
}

async function listDevices() {
  if (!state.accessToken) {
    $('deviceResult').textContent = '未登录或缺少 access_token';
    return;
  }
  try {
    const res = await fetch(`${state.apiBase}/devices`, {
      method: 'GET',
      headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
  if (!state.accessToken) return;
  try {
    const res = await fetch(`${state.apiBase}/devices/${encodeURIComponent(deviceId)}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
    hasAccessToken: !!state.accessToken,
    hasRefreshToken: !!state.refreshToken,
  });
}

// ========================================
// Messages API
// ========================================

// Send message
async function sendMessage() {
  if (!state.accessToken) { $('msgSendResFmt').textContent = '未登录'; return; }
  
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
      'Authorization': `Bearer ${state.accessToken}` 
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
  if (!state.accessToken) { $('msgGetResFmt').textContent = '未登录'; return; }
  
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
    headers: { 'Authorization': `Bearer ${state.accessToken}` },
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
  if (!state.accessToken) { $('msgDeleteResFmt').textContent = '未登录'; return; }
  if (!uuid) { $('msgDeleteResFmt').textContent = '消息UUID必填'; return; }

  const req = {
    method: 'DELETE',
    url: `${state.messagesBase}/delete`,
    headers: { 
      'Content-Type': 'application/json', 
      'Authorization': `Bearer ${state.accessToken}` 
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
  if (!state.accessToken) { $('msgRecallResFmt').textContent = '未登录'; return; }
  if (!uuid) { $('msgRecallResFmt').textContent = '消息UUID必填'; return; }

  const req = {
    method: 'POST',
    url: `${state.messagesBase}/recall`,
    headers: { 
      'Content-Type': 'application/json', 
      'Authorization': `Bearer ${state.accessToken}` 
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

// 计算文件SHA-256哈希（采样策略，避免大文件内存溢出）
async function calculateSHA256(file) {
  try {
    const SAMPLE_SIZE = 10 * 1024 * 1024; // 10MB
    
    // 文件元信息
    const metadata = `${file.name}|${file.size}|${file.lastModified}|${file.type}`;
    const metadataBuffer = new TextEncoder().encode(metadata);
    
    let dataToHash;
    
    if (file.size <= SAMPLE_SIZE * 3) {
      // 小文件（< 30MB）：计算完整哈希
      console.log('小文件，计算完整哈希');
      dataToHash = await file.arrayBuffer();
    } else {
      // 大文件：采样哈希策略
      console.log('大文件，使用采样哈希策略');
      const chunks = [];
      
      // 读取开头10MB
      const startBlob = file.slice(0, SAMPLE_SIZE);
      chunks.push(new Uint8Array(await startBlob.arrayBuffer()));
      
      // 读取中间10MB
      const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
      const middleBlob = file.slice(middleStart, middleStart + SAMPLE_SIZE);
      chunks.push(new Uint8Array(await middleBlob.arrayBuffer()));
      
      // 读取结尾10MB
      const endBlob = file.slice(file.size - SAMPLE_SIZE, file.size);
      chunks.push(new Uint8Array(await endBlob.arrayBuffer()));
      
      // 合并所有数据
      const totalLength = metadataBuffer.length + chunks.reduce((sum, chunk) => sum + chunk.length, 0);
      dataToHash = new Uint8Array(totalLength);
      let offset = 0;
      
      dataToHash.set(metadataBuffer, offset);
      offset += metadataBuffer.length;
      
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

  if (!state.accessToken) {
    $('fileUploadResFmt').textContent = '未登录，请先登录';
    return;
  }

  try {
    updateProgress(0, '准备上传...');
    
    // 1. 计算文件哈希
    updateProgress(10, '正在计算文件哈希...');
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
        'Authorization': `Bearer ${state.accessToken}`
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
          'Authorization': `Bearer ${state.accessToken}`
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
    <div id="file-item-${index}" style="padding: 10px; margin: 5px 0; background: ${file.instant ? '#e8f5e9' : '#f5f5f5'}; border-radius: 5px;">
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
          ${file.preview_support === 'inline_preview' ? '🖼️ 新标签页预览' : '📥 下载'}
        </a>
        <button onclick="toggleInlinePreview(${index}, '${file.file_url.replace(/'/g, "\\'")}', '${file.filename.replace(/'/g, "\\'")}', ${file.file_size || 0})" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em;">📺 在线查看</button>
        <button onclick="copyToClipboard('${file.file_url}')" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em;">复制URL</button>
        <button onclick="removeUploadedFile(${index})" style="margin-left: 10px; padding: 2px 8px; font-size: 0.85em; background: #f44336; color: white;">删除记录</button>
      </div>
      <div id="inline-preview-${index}" style="display: none; margin-top: 15px; padding: 10px; background: white; border-radius: 5px; border: 2px solid #007bff;"></div>
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
// 预签名URL管理
// =============================================

// 获取预签名URL（带缓存和过期检查，自动重试，15GB阈值）
async function getPresignedUrl(uuid, filename, retryCount = 0) {
  const MAX_RETRIES = 2;
  
  // 0. 从localStorage重新读取最新的Token（防止刷新后未同步）
  state.accessToken = localStorage.getItem('accessToken') || '';
  state.refreshToken = localStorage.getItem('refreshToken') || '';
  
  // 检查Token是否存在
  if (!state.accessToken) {
    throw new Error('未登录，请先登录');
  }
  
  // 检查Token是否过期
  const claims = decodeJwt(state.accessToken);
  if (claims && claims.exp && claims.exp * 1000 < Date.now()) {
    // Token已过期，尝试刷新
    console.log('🔄 Access Token已过期，尝试刷新...');
    try {
      await refreshAccessToken();
      // 刷新成功后，重新读取token
      state.accessToken = localStorage.getItem('accessToken') || '';
      if (!state.accessToken) {
        throw new Error('Token刷新失败');
      }
      console.log('✅ Token刷新成功，继续执行');
    } catch (e) {
      console.error('❌ Token刷新失败:', e);
      throw new Error('登录已过期，请重新登录');
    }
  }
  
  // 1. 检查缓存是否有效（过期前5分钟需要刷新）
  const cached = state.presignedUrls[uuid];
  if (cached && cached.expiresAt) {
    const expiresTime = new Date(cached.expiresAt);
    const now = new Date();
    const remainingMs = expiresTime - now;
    
    // 还有5分钟以上过期，直接使用缓存
    if (remainingMs > 5 * 60 * 1000) {
      console.log(`✅ 使用缓存的预签名URL，剩余时间: ${Math.floor(remainingMs / 60000)} 分钟`);
      return cached.url;
    } else if (remainingMs > 0) {
      // 还未过期但少于5分钟，提示用户
      const minutes = Math.floor(remainingMs / 60000);
      console.log(`⚠️ 文件访问链接将在 ${minutes} 分钟后过期，正在刷新链接...`);
    } else {
      // 已过期，清除缓存
      console.log('🔄 缓存的预签名URL已过期，重新获取...');
      delete state.presignedUrls[uuid];
    }
  }
  
  // 2. 检测文件大小，判断是否超大文件（15GB阈值）
  const fileInfo = state.uploadedFiles.find(f => 
    f.file_url && f.file_url.includes(uuid)
  );
  
  const LARGE_FILE_THRESHOLD = 15 * 1024 * 1024 * 1024; // 15GB
  const isLargeFile = fileInfo && fileInfo.file_size > LARGE_FILE_THRESHOLD;
  
  let endpoint, body;
  
  if (isLargeFile) {
    // 超大文件：提示用户输入预计下载时间
    const hours = prompt(
      `检测到大文件（${formatFileSize(fileInfo.file_size)}）\n请输入预计下载时间（小时，最少3，最多168）:`,
      '6'
    );
    
    if (!hours || parseInt(hours) < 3) {
      throw new Error('下载已取消或时间无效（最少3小时）');
    }
    
    const estimatedSeconds = Math.min(parseInt(hours) * 3600, 604800); // 最多7天
    endpoint = `${state.storageBase}/file/${uuid}/presigned-url/extended`;
    body = {
      operation: 'download',
      estimated_download_time: estimatedSeconds
    };
  } else {
    // 普通文件：3小时有效期
    endpoint = `${state.storageBase}/file/${uuid}/presigned-url`;
    body = {
      operation: 'download'
    };
  }
  
  // 5. 请求预签名URL（确保使用最新的token）
  const currentToken = localStorage.getItem('accessToken') || state.accessToken;
  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${currentToken}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(body)
  });
  
  // 改进错误处理（支持自动重试）
  if (!response.ok) {
    let errorMessage = '获取下载链接失败';
    let shouldRetry = false;
    
    try {
      const error = await response.json();
      errorMessage = error.error || errorMessage;
    } catch (e) {
      // 如果响应不是JSON（如HTML错误页）
      if (response.status === 401) {
        // Token过期，尝试刷新并重试
        if (retryCount < MAX_RETRIES) {
          console.log(`🔄 检测到401错误，刷新Token并重试 (${retryCount + 1}/${MAX_RETRIES})...`);
          try {
            await refreshAccessToken();
            // Token刷新成功，重新读取并递归重试
            state.accessToken = localStorage.getItem('accessToken') || '';
            state.refreshToken = localStorage.getItem('refreshToken') || '';
            console.log('✅ Token刷新成功，正在重试请求...');
            return await getPresignedUrl(uuid, filename, retryCount + 1);
          } catch (refreshError) {
            console.error('❌ Token刷新失败:', refreshError);
            errorMessage = '登录已过期，请重新登录';
            // 清空过期token
            state.accessToken = '';
            state.refreshToken = '';
            localStorage.removeItem('accessToken');
            localStorage.removeItem('refreshToken');
            renderLocalState();
          }
        } else {
          console.error('❌ 已达到最大重试次数');
          errorMessage = '登录已过期，请重新登录';
          // 清空过期token
          state.accessToken = '';
          state.refreshToken = '';
          localStorage.removeItem('accessToken');
          localStorage.removeItem('refreshToken');
          renderLocalState();
        }
      } else if (response.status === 403) {
        errorMessage = '无权访问此文件';
      } else if (response.status === 404) {
        errorMessage = '文件不存在';
      } else {
        const text = await response.text().catch(() => '');
        errorMessage = `服务器错误 (${response.status})${text ? ': ' + text.substring(0, 100) : ''}`;
      }
    }
    
    throw new Error(errorMessage);
  }
  
  const data = await response.json();
  
  // 4. 缓存预签名URL
  state.presignedUrls[uuid] = {
    url: data.presigned_url,
    expiresAt: data.expires_at,
    cachedAt: new Date().toISOString()
  };
  
  console.log(`✅ 获取新的预签名URL，过期时间: ${data.expires_at}`);
  if (data.warning) {
    console.warn(`⚠️ ${data.warning}`);
  }
  
  return data.presigned_url;
}

// 视频/图片加载错误时自动重新获取URL
async function handleMediaError(event, uuid, filename, mediaElement) {
  console.warn('⚠️ 媒体加载失败，可能是URL过期');
  
  // 检查是否是网络错误还是URL过期
  const cached = state.presignedUrls[uuid];
  if (cached && cached.expiresAt) {
    const expiresTime = new Date(cached.expiresAt);
    const now = new Date();
    
    if (now > expiresTime) {
      console.log('🔄 检测到URL已过期，自动重新获取...');
      try {
        // 清除过期的缓存
        delete state.presignedUrls[uuid];
        
        // 重新获取预签名URL
        const newUrl = await getPresignedUrl(uuid, filename);
        
        // 更新媒体元素的src
        if (mediaElement) {
          mediaElement.src = newUrl;
          console.log('✅ URL已更新，正在重新加载...');
        }
        
        return newUrl;
      } catch (error) {
        console.error('❌ 重新获取URL失败:', error);
        throw error;
      }
    }
  }
  
  // 如果不是URL过期，可能是其他网络问题
  throw new Error('媒体加载失败，请检查网络连接');
}

// =============================================
// 文件预览功能
// =============================================

// 预览文件（根据文件名后缀判断类型，支持UUID预签名URL）
async function previewFile(filename = '') {
  const urlInput = document.getElementById('preview_url');
  const previewContainer = document.getElementById('filePreview');
  const url = urlInput.value.trim();
  
  if (!url) {
    previewContainer.innerHTML = '<p style="color: #999;">请输入文件URL</p>';
    return;
  }
  
  // 判断是否是UUID格式的URL（需要获取预签名URL）
  const isUuidUrl = url.includes('/api/storage/file/');
  
  // 确定用于判断类型的字符串（优先使用filename，否则使用URL）
  const checkString = filename || url;
  const lowerStr = checkString.toLowerCase();
  
  // 根据文件名后缀判断文件类型
  if (lowerStr.match(/\.(jpg|jpeg|png|gif|webp)$/i)) {
    // 图片预览
    if (isUuidUrl) {
      previewContainer.innerHTML = '<p>正在获取图片链接...</p>';
      try {
        const uuid = url.split('/').pop();
        const presignedUrl = await getPresignedUrl(uuid, filename);
        
        previewContainer.innerHTML = `
          <div>
            <h4 style="margin-bottom: 10px;">图片预览（直连MinIO）</h4>
            <img src="${presignedUrl}" alt="图片预览" 
                 style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);" 
                 onerror="this.parentElement.innerHTML='<p style=color:red>图片加载失败，链接可能已过期</p>'" />
            <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
              ${filename ? `文件名: ${filename}<br/>` : ''}
              UUID: ${uuid}<br/>
              ✅ 客户端直连MinIO，后端零压力<br/>
              ⏰ 链接3小时内有效
            </div>
            <div style="margin-top: 10px;">
              <a href="${presignedUrl}" download="${filename || 'image.jpg'}" 
                 style="padding: 8px 16px; background: #28a745; color: white; text-decoration: none; border-radius: 5px; cursor: pointer;">
                📥 下载图片
              </a>
            </div>
          </div>
        `;
      } catch (error) {
        previewContainer.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
      }
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
    // 视频预览 - 必须使用预签名URL
    if (isUuidUrl) {
      previewContainer.innerHTML = '<p>正在获取视频流式播放链接...</p>';
      try {
        const uuid = url.split('/').pop();
        const presignedUrl = await getPresignedUrl(uuid, filename);
        
        previewContainer.innerHTML = `
          <div>
            <h4 style="margin-bottom: 10px;">视频在线流式播放</h4>
            <video controls style="max-width: 100%; max-height: 600px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);">
              <source src="${presignedUrl}" type="video/mp4">
              您的浏览器不支持视频播放
            </video>
            <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
              ${filename ? `文件名: ${filename}<br/>` : ''}
              UUID: ${uuid}<br/>
              ✅ 真正的流式播放（支持拖动进度条）<br/>
              ✅ 支持Range请求（按需加载视频片段）<br/>
              ✅ 客户端直连MinIO，后端零压力<br/>
              ⏰ 链接3小时内有效
            </div>
            <div style="margin-top: 10px;">
              <a href="${presignedUrl}" download="${filename || 'video.mp4'}" 
                 style="padding: 8px 16px; background: #28a745; color: white; text-decoration: none; border-radius: 5px; cursor: pointer;">
                📥 下载视频
              </a>
            </div>
          </div>
        `;
      } catch (error) {
        previewContainer.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
      }
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
      previewContainer.innerHTML = '<p>正在获取PDF链接...</p>';
      try {
        const uuid = url.split('/').pop();
        const presignedUrl = await getPresignedUrl(uuid, filename);
        
        previewContainer.innerHTML = `
          <div>
            <h4 style="margin-bottom: 10px;">PDF预览</h4>
            <iframe src="${presignedUrl}" 
                    style="width: 100%; height: 600px; border: none; border-radius: 5px;"></iframe>
            <div style="margin-top: 10px; font-size: 0.9em; color: #666;">
              ${filename ? `文件名: ${filename}<br/>` : ''}
              UUID: ${uuid}<br/>
              ⏰ 链接3小时内有效
            </div>
            <div style="margin-top: 10px;">
              <a href="${presignedUrl}" download="${filename || 'document.pdf'}" 
                 style="padding: 8px 16px; background: #28a745; color: white; text-decoration: none; border-radius: 5px; cursor: pointer;">
                📥 下载PDF
              </a>
            </div>
          </div>
        `;
      } catch (error) {
        previewContainer.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
      }
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
      previewContainer.innerHTML = '<p>正在获取文件链接...</p>';
      try {
        const uuid = url.split('/').pop();
        const presignedUrl = await getPresignedUrl(uuid, filename);
        
        previewContainer.innerHTML = `
          <div>
            <h4 style="margin-bottom: 10px;">文件信息 (UUID访问)</h4>
            <p>文件名: ${filename || '未知'}</p>
            <p style="color: #666; word-break: break-all;">UUID: ${uuid}</p>
            <p style="color: #4caf50;">🔗 已获取临时访问链接</p>
            <div style="margin-top: 15px;">
              <a href="${presignedUrl}" download="${filename || uuid}" 
                 style="padding: 10px 20px; background: #28a745; color: white; text-decoration: none; border-radius: 5px; cursor: pointer;">
                📥 下载文件
              </a>
            </div>
          </div>
        `;
      } catch (error) {
        previewContainer.innerHTML = `<p style="color: red;">加载失败: ${error.message}</p>`;
      }
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

// 从已上传文件列表预览（已弃用，使用 toggleInlinePreview）
function previewUploadedFile(url, filename) {
  document.getElementById('preview_url').value = url;
  previewFile(filename);
  // 滚动到预览区域
  document.getElementById('filePreview').scrollIntoView({ behavior: 'smooth', block: 'center' });
}

// 切换内联预览（直接在文件列表项下方显示）
async function toggleInlinePreview(index, url, filename, fileSize) {
  const previewContainer = document.getElementById(`inline-preview-${index}`);
  const fileItem = document.getElementById(`file-item-${index}`);
  
  if (!previewContainer || !fileItem) return;
  
  // 如果已经展开，则收起
  if (previewContainer.style.display !== 'none') {
    previewContainer.style.display = 'none';
    previewContainer.innerHTML = '';
    return;
  }
  
  // 关闭其他所有预览
  document.querySelectorAll('[id^="inline-preview-"]').forEach(el => {
    el.style.display = 'none';
    el.innerHTML = '';
  });
  
  // 显示当前预览
  previewContainer.style.display = 'block';
  previewContainer.innerHTML = '<p style="color: #666;">⏳ 正在获取访问链接...</p>';
  
  // 判断是否是UUID格式的URL
  const isUuidUrl = url.includes('/api/storage/file/');
  const lowerFilename = filename.toLowerCase();
  
  try {
    let presignedUrl = url;
    let uuid = '';
    
    // 如果是UUID URL，获取预签名URL
    if (isUuidUrl) {
      uuid = url.split('/').pop();
      try {
        presignedUrl = await getPresignedUrl(uuid, filename);
        if (!presignedUrl) {
          throw new Error('获取访问链接失败');
        }
      } catch (error) {
        // 特殊处理登录过期的情况
        if (error.message.includes('登录已过期') || error.message.includes('未登录')) {
          previewContainer.innerHTML = `
            <div style="color: #ff6b6b; padding: 20px; text-align: center;">
              <p style="font-size: 1.1em; margin-bottom: 10px;">🔒 ${error.message}</p>
              <p style="font-size: 0.9em; color: #666; margin-bottom: 15px;">请重新登录后继续</p>
              <button onclick="document.getElementById('inline-preview-${index}').style.display='none'; window.scrollTo({top: 0, behavior: 'smooth'});" 
                      style="padding: 8px 16px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
                返回顶部登录
              </button>
            </div>
          `;
          return;
        }
        throw error;
      }
    }
    
    // 根据文件类型生成预览内容
    if (lowerFilename.match(/\.(jpg|jpeg|png|gif|webp)$/i)) {
      // 图片预览（支持自动重试）
      previewContainer.innerHTML = `
        <div style="text-align: center;">
          <div style="font-weight: bold; margin-bottom: 10px; color: #007bff;">📷 图片预览</div>
          <img id="img-${index}" src="${presignedUrl}" alt="${filename}" 
               style="max-width: 100%; max-height: 500px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.15);" />
          ${isUuidUrl ? `
            <div style="margin-top: 10px; font-size: 0.85em; color: #666;">
              ✅ 客户端直连MinIO | ⏰ 链接3小时有效 | 🔄 URL过期自动刷新
            </div>
          ` : ''}
          <div style="margin-top: 10px;">
            <a href="${presignedUrl}" download="${filename}" 
               style="padding: 6px 12px; background: #28a745; color: white; text-decoration: none; border-radius: 4px; font-size: 0.9em;">
              📥 下载
            </a>
            <button onclick="document.getElementById('inline-preview-${index}').style.display='none'" 
                    style="margin-left: 10px; padding: 6px 12px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
              关闭预览
            </button>
          </div>
        </div>
      `;
      
      // 添加错误处理（自动重试）
      if (isUuidUrl) {
        const imgElement = document.getElementById(`img-${index}`);
        if (imgElement) {
          imgElement.onerror = async () => {
            try {
              imgElement.onerror = null; // 防止循环
              console.log('🔄 图片加载失败，尝试重新获取URL...');
              const newUrl = await handleMediaError(null, uuid, filename, imgElement);
              imgElement.src = newUrl;
            } catch (error) {
              imgElement.parentElement.innerHTML = '<p style="color:red">图片加载失败: ' + error.message + '</p>';
            }
          };
        }
      }
    } else if (lowerFilename.match(/\.(mp4|webm|ogg|mov)$/i)) {
      // 视频预览（支持自动重试）
      previewContainer.innerHTML = `
        <div>
          <div style="font-weight: bold; margin-bottom: 10px; color: #007bff;">🎬 视频在线播放</div>
          <video id="video-${index}" controls style="width: 100%; max-height: 500px; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.15);">
            <source src="${presignedUrl}" type="video/mp4">
            您的浏览器不支持视频播放
          </video>
          ${isUuidUrl ? `
            <div style="margin-top: 10px; font-size: 0.85em; color: #666;">
              ✅ 流式播放（可拖动进度条） | ✅ Range请求 | ✅ 直连MinIO | ⏰ 链接3小时有效 | 🔄 URL过期自动刷新
            </div>
          ` : ''}
          <div style="margin-top: 10px;">
            <a href="${presignedUrl}" download="${filename}" 
               style="padding: 6px 12px; background: #28a745; color: white; text-decoration: none; border-radius: 4px; font-size: 0.9em;">
              📥 下载视频
            </a>
            <button onclick="document.getElementById('inline-preview-${index}').style.display='none'; var v=document.getElementById('video-${index}'); if(v)v.pause();" 
                    style="margin-left: 10px; padding: 6px 12px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
              关闭预览
            </button>
          </div>
        </div>
      `;
      
      // 添加错误处理（自动重试）
      if (isUuidUrl) {
        const videoElement = document.getElementById(`video-${index}`);
        if (videoElement) {
          videoElement.onerror = async () => {
            try {
              videoElement.onerror = null; // 防止循环
              console.log('🔄 视频加载失败，尝试重新获取URL...');
              const currentTime = videoElement.currentTime || 0; // 保存当前播放位置
              const newUrl = await handleMediaError(null, uuid, filename, videoElement);
              videoElement.src = newUrl;
              videoElement.currentTime = currentTime; // 恢复播放位置
              videoElement.load();
            } catch (error) {
              videoElement.parentElement.innerHTML = '<p style="color:red">视频加载失败: ' + error.message + '</p>';
            }
          };
        }
      }
    } else if (lowerFilename.match(/\.pdf$/i)) {
      // PDF预览
      previewContainer.innerHTML = `
        <div>
          <div style="font-weight: bold; margin-bottom: 10px; color: #007bff;">📄 PDF预览</div>
          <iframe src="${presignedUrl}" 
                  style="width: 100%; height: 500px; border: none; border-radius: 5px; box-shadow: 0 2px 8px rgba(0,0,0,0.15);"></iframe>
          ${isUuidUrl ? `
            <div style="margin-top: 10px; font-size: 0.85em; color: #666;">
              ⏰ 链接3小时有效
            </div>
          ` : ''}
          <div style="margin-top: 10px;">
            <a href="${presignedUrl}" download="${filename}" 
               style="padding: 6px 12px; background: #28a745; color: white; text-decoration: none; border-radius: 4px; font-size: 0.9em;">
              📥 下载PDF
            </a>
            <button onclick="document.getElementById('inline-preview-${index}').style.display='none'" 
                    style="margin-left: 10px; padding: 6px 12px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
              关闭预览
            </button>
          </div>
        </div>
      `;
    } else {
      // 其他文件类型
      previewContainer.innerHTML = `
        <div>
          <div style="font-weight: bold; margin-bottom: 10px; color: #007bff;">📦 文件信息</div>
          <p>文件名: ${filename}</p>
          ${fileSize > 0 ? `<p>大小: ${formatFileSize(fileSize)}</p>` : ''}
          ${isUuidUrl ? `<p style="color: #666; word-break: break-all; font-size: 0.9em;">UUID: ${uuid}</p>` : ''}
          <p style="color: #999;">该文件类型不支持在线预览</p>
          <div style="margin-top: 10px;">
            <a href="${presignedUrl}" download="${filename}" 
               style="padding: 6px 12px; background: #28a745; color: white; text-decoration: none; border-radius: 4px; font-size: 0.9em;">
              📥 下载文件
            </a>
            <button onclick="document.getElementById('inline-preview-${index}').style.display='none'" 
                    style="margin-left: 10px; padding: 6px 12px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
              关闭
            </button>
          </div>
        </div>
      `;
    }
    
    // 平滑滚动到预览区域
    setTimeout(() => {
      previewContainer.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }, 100);
    
  } catch (error) {
    console.error('预览失败:', error);
    
    // 友好的错误提示
    let errorHtml = '';
    if (error.message.includes('登录已过期') || error.message.includes('未登录')) {
      errorHtml = `
        <div style="color: #ff6b6b; padding: 20px; text-align: center;">
          <p style="font-size: 1.1em; margin-bottom: 10px;">🔒 ${error.message}</p>
          <p style="font-size: 0.9em; color: #666; margin-bottom: 15px;">请重新登录后继续</p>
          <button onclick="document.getElementById('inline-preview-${index}').style.display='none'; window.scrollTo({top: 0, behavior: 'smooth'});" 
                  style="padding: 8px 16px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
            返回顶部登录
          </button>
        </div>
      `;
    } else if (error.message.includes('无权访问')) {
      errorHtml = `
        <div style="color: #ff9800; padding: 20px; text-align: center;">
          <p style="font-size: 1.1em; margin-bottom: 10px;">🚫 ${error.message}</p>
          <p style="font-size: 0.9em; color: #666;">您没有访问此文件的权限</p>
        </div>
      `;
    } else if (error.message.includes('文件不存在')) {
      errorHtml = `
        <div style="color: #f44336; padding: 20px; text-align: center;">
          <p style="font-size: 1.1em; margin-bottom: 10px;">❌ ${error.message}</p>
          <p style="font-size: 0.9em; color: #666;">文件可能已被删除</p>
        </div>
      `;
    } else {
      errorHtml = `
        <div style="color: #f44336; padding: 20px;">
          <p style="font-size: 1.1em; margin-bottom: 10px;">❌ 加载失败</p>
          <p style="font-size: 0.9em; color: #666; word-break: break-word;">${error.message}</p>
        </div>
      `;
    }
    
    previewContainer.innerHTML = `
      ${errorHtml}
      <div style="text-align: center; margin-top: 10px;">
        <button onclick="document.getElementById('inline-preview-${index}').style.display='none'" 
                style="padding: 6px 12px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9em;">
          关闭
        </button>
      </div>
    `;
  }
}

window.addEventListener('DOMContentLoaded', init);