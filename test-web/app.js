const $ = (id) => document.getElementById(id);
const pretty = (obj) => JSON.stringify(obj, null, 2);

const state = {
  apiBase: localStorage.getItem('apiBase') || 'http://localhost:8080/api/auth',
  friendsBase: localStorage.getItem('friendsApiBase') || 'http://localhost:8080/api/friends',
  messagesBase: localStorage.getItem('messagesApiBase') || 'http://localhost:8080/api/messages',
  accessToken: localStorage.getItem('accessToken') || '',
  refreshToken: localStorage.getItem('refreshToken') || '',
};

function init() {
  $('apiBase').value = state.apiBase;
  $('friendsApiBase').value = state.friendsBase;
  $('messagesApiBase').value = state.messagesBase;
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
    $('profile').textContent = '缺少 refresh_token';
    return;
  }
  try {
    const res = await fetch(`${state.apiBase}/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: state.refreshToken }),
    });
    const data = await res.json();
    if (res.ok && data.access_token) {
      state.accessToken = data.access_token;
      localStorage.setItem('accessToken', state.accessToken);
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

window.addEventListener('DOMContentLoaded', init);