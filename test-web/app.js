/**
 * HuanVae Chat - 前端应用
 * 完整功能：认证、好友、消息、群聊、文件存储
 */

// ==========================================
// 配置和全局状态
// ==========================================

const BASE_URL = 'http://localhost';  // 通过 Nginx 代理

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
};

// ==========================================
// 工具函数
// ==========================================

// API 请求封装
async function api(path, { method = 'GET', body, formData, token = state.accessToken } = {}) {
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

// 计算文件 SHA-256 哈希
async function calculateSHA256(file) {
  const SAMPLE_SIZE = 10 * 1024 * 1024;
  const metadata = `${file.name}|${file.size}|${file.lastModified}|${file.type}`;
  const metadataBuffer = new TextEncoder().encode(metadata);
  
  let dataToHash;
  
  if (file.size <= SAMPLE_SIZE * 3) {
    dataToHash = await file.arrayBuffer();
  } else {
    const chunks = [];
    chunks.push(new Uint8Array(await file.slice(0, SAMPLE_SIZE).arrayBuffer()));
    const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
    chunks.push(new Uint8Array(await file.slice(middleStart, middleStart + SAMPLE_SIZE).arrayBuffer()));
    chunks.push(new Uint8Array(await file.slice(file.size - SAMPLE_SIZE, file.size).arrayBuffer()));
    
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
// 认证功能
// ==========================================

// 检查登录状态
function checkAuth() {
  if (!state.accessToken) {
    openModal('authModal');
    return false;
  }
  
  const claims = decodeJwt(state.accessToken);
  if (!claims || claims.exp * 1000 < Date.now()) {
    // Token 过期，尝试刷新
    if (state.refreshToken) {
      refreshTokenRequest();
    } else {
      openModal('authModal');
    }
    return false;
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
    initApp();
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
    const result = await api('/api/auth/refresh', {
      method: 'POST',
      body: { refresh_token: state.refreshToken },
      token: null
    });
    
    state.accessToken = result.access_token;
    localStorage.setItem('accessToken', result.access_token);
  } catch {
    // 刷新失败，需要重新登录
    state.accessToken = '';
    state.refreshToken = '';
    localStorage.removeItem('accessToken');
    localStorage.removeItem('refreshToken');
    openModal('authModal');
  }
}

// 登出
async function logout() {
  try {
    await api('/api/auth/logout', { method: 'POST' });
  } catch {}
  
  state.accessToken = '';
  state.refreshToken = '';
  state.currentUser = null;
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
  // 合并好友和群聊作为会话
  const conversations = [];
  
  // 好友会话
  for (const f of state.friends) {
    conversations.push({
      type: 'friend',
      id: f.friend_id,
      name: f.friend_nickname || f.friend_id,
      lastMessage: '',
      time: f.add_time
    });
  }
  
  // 群聊会话
  for (const g of state.groups) {
    conversations.push({
      type: 'group',
      id: g.group_id,
      name: g.group_name,
      lastMessage: '',
      time: g.created_at,
      memberCount: g.member_count
    });
  }
  
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
    const isActive = state.currentChat?.type === c.type && state.currentChat?.id === c.id;
    return `
      <div class="conversation-item ${isActive ? 'active' : ''}" 
           onclick="openChat('${c.type}', '${c.id}', '${c.name}')">
        <div class="item-avatar">${c.type === 'group' ? '👥' : '👤'}</div>
        <div class="item-info">
          <div class="item-name">${c.name}</div>
          <div class="item-preview">${c.lastMessage || (c.type === 'group' ? `${c.memberCount || 0}人` : '')}</div>
        </div>
        <div class="item-meta">
          <div class="item-time">${formatTime(c.time)}</div>
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
  document.getElementById('chatAvatar').innerHTML = type === 'group' ? '👥' : '👤';
  
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
  loadMessages();
  
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

// 加载消息
async function loadMessages() {
  if (!state.currentChat) return;
  
  const { type, id } = state.currentChat;
  const container = document.getElementById('messageList');
  
  try {
    let messages;
    
    if (type === 'friend') {
      const result = await api(`/api/messages?friend_id=${encodeURIComponent(id)}&limit=50`);
      messages = result.messages || [];
    } else {
      const result = await api(`/api/group-messages?group_id=${encodeURIComponent(id)}&limit=50`);
      messages = result.data?.messages || [];
    }
    
    state.messages[`${type}-${id}`] = messages;
    renderMessages(messages);
  } catch (err) {
    console.error('加载消息失败:', err);
    container.innerHTML = '<div class="empty-state"><p>加载消息失败</p></div>';
  }
}

function renderMessages(messages) {
  const container = document.getElementById('messageList');
  const claims = decodeJwt(state.accessToken);
  const myId = claims?.sub;
  
  if (messages.length === 0) {
    container.innerHTML = '<div class="empty-state"><div class="empty-icon">💬</div><p>暂无消息</p></div>';
    return;
  }
  
  // 翻转消息顺序：后端返回最新在前，前端需要最新在后（底部）
  const sortedMessages = [...messages].reverse();
  
  container.innerHTML = sortedMessages.map(msg => {
    const isSelf = msg.sender_id === myId;
    const hasFile = msg.file_uuid && msg.file_uuid !== 'null';
    
    // 获取头像
    let avatarHtml = '👤';
    if (isSelf) {
      // 自己的头像
      if (state.userProfile?.avatar_url) {
        avatarHtml = `<img src="${state.userProfile.avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`;
      }
    } else if (state.currentChat?.type === 'friend') {
      // 好友头像：从好友列表查找
      const friend = state.friends.find(f => f.friend_id === msg.sender_id);
      if (friend?.friend_avatar_url) {
        avatarHtml = `<img src="${friend.friend_avatar_url}" alt="头像" class="avatar-img" onerror="this.parentElement.innerHTML='👤'">`;
      }
    }
    
    let contentHtml = `<div class="message-text">${escapeHtml(msg.message_content)}</div>`;
    
    // 文件消息
    if (hasFile) {
      if (msg.message_type === 'image') {
        contentHtml += `<img class="message-image" src="" alt="图片" onclick="previewFile('${msg.file_uuid}')" data-uuid="${msg.file_uuid}">`;
      } else if (msg.message_type === 'video') {
        contentHtml += `<video class="message-video" controls data-uuid="${msg.file_uuid}"></video>`;
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
    
    return `
      <div class="message ${isSelf ? 'self' : ''}">
        <div class="message-avatar">${avatarHtml}</div>
        <div class="message-body">
          ${!isSelf && state.currentChat?.type === 'group' ? `<div class="message-sender">${msg.sender_id}</div>` : ''}
          <div class="message-bubble">${contentHtml}</div>
          <div class="message-time">${formatTime(msg.send_time)}</div>
        </div>
      </div>
    `;
  }).join('');
  
  // 滚动到底部
  const msgContainer = document.getElementById('messageContainer');
  msgContainer.scrollTop = msgContainer.scrollHeight;
  
  // 加载文件预览
  loadFilePreviewsInMessages();
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// 加载消息中的文件预览
async function loadFilePreviewsInMessages() {
  const images = document.querySelectorAll('.message-image[data-uuid]');
  const videos = document.querySelectorAll('.message-video[data-uuid]');
  
  for (const img of images) {
    const uuid = img.dataset.uuid;
    try {
      const url = await getFilePresignedUrl(uuid);
      if (url) img.src = url;
    } catch {}
  }
  
  for (const video of videos) {
    const uuid = video.dataset.uuid;
    try {
      const url = await getFilePresignedUrl(uuid);
      if (url) video.src = url;
    } catch {}
  }
}

async function getFilePresignedUrl(uuid) {
  // 根据当前聊天类型选择 API
  const endpoint = state.currentChat?.type === 'group'
    ? `/api/storage/file/${uuid}/presigned_url`
    : `/api/storage/friends_file/${uuid}/presigned_url`;
  
  const result = await api(endpoint, { method: 'POST', body: { operation: 'download' } });
  return result.presigned_url;
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

// 发送文件消息
async function sendFileMessage(files) {
  if (!files || !files[0] || !state.currentChat) return;
  
  const file = files[0];
  const { type, id } = state.currentChat;
  
  try {
    showToast('正在上传...', 'info');
    
    // 计算哈希
    const file_hash = await calculateSHA256(file);
    
    // 确定文件类型
    let file_type = 'user_document';
    if (file.type.startsWith('image/')) file_type = 'user_image';
    else if (file.type.startsWith('video/')) file_type = 'user_video';
    
    // 请求上传
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
    
    if (uploadInfo.instant_upload) {
      showToast('发送成功（秒传）', 'success');
      loadMessages();
      return;
    }
    
    // 上传文件
    const formData = new FormData();
    formData.append('file', file);
    
    const uploadResult = await fetch(uploadInfo.upload_url, {
      method: 'POST',
      body: formData
    });
    
    if (!uploadResult.ok) throw new Error('上传失败');
    
    showToast('发送成功', 'success');
    loadMessages();
  } catch (err) {
    showToast(err.message, 'error');
  }
  
  // 清空文件输入
  document.getElementById('chatFileInput').value = '';
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
    
    const members = await loadGroupMembers(id);
    const claims = decodeJwt(state.accessToken);
    const myMember = members.find(m => m.user_id === claims?.sub);
    const isAdmin = myMember?.role === 'owner' || myMember?.role === 'admin';
    
    content.innerHTML = `
      <div class="info-section" style="text-align: center;">
        <div class="info-avatar">👥</div>
        <div class="info-name">${name}</div>
        <div class="info-id">${members.length}人</div>
      </div>
      
      <div class="info-section">
        <h4>群成员</h4>
        <div class="member-list">
          ${members.map(m => `
            <div class="member-item">
              <div class="member-avatar">👤</div>
              <div class="member-name">${m.user_id}</div>
              <span class="member-role ${m.role}">${m.role === 'owner' ? '群主' : m.role === 'admin' ? '管理员' : ''}</span>
            </div>
          `).join('')}
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
  
  try {
    // 1. 计算哈希
    progressText.textContent = '计算文件哈希...';
    progressFill.style.width = '10%';
    
    const file_hash = await calculateSHA256(file);
    
    // 2. 确定参数
    let file_type = 'user_document';
    if (file.type.startsWith('image/')) file_type = 'user_image';
    else if (file.type.startsWith('video/')) file_type = 'user_video';
    
    const storage_location = document.getElementById('uploadLocation').value;
    const related_id = document.getElementById('uploadRelatedId')?.value;
    const force_upload = document.getElementById('forceUpload').checked;
    
    // 3. 请求上传
    progressText.textContent = '请求上传...';
    progressFill.style.width = '20%';
    
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
    
    // 5. 上传文件
    progressText.textContent = '上传中...';
    progressFill.style.width = '50%';
    
    const formData = new FormData();
    formData.append('file', file);
    
    const uploadResult = await fetch(uploadInfo.upload_url, {
      method: 'POST',
      body: formData
    });
    
    if (!uploadResult.ok) throw new Error('上传失败');
    
    progressFill.style.width = '100%';
    progressText.textContent = '上传成功！';
    showToast('文件上传成功', 'success');
    loadMyFiles();
    
  } catch (err) {
    progressFill.style.width = '0%';
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
  
  container.innerHTML = conversations.map(c => `
    <div class="conversation-item" onclick="openChat('${c.type}', '${c.id}', '${c.name}')">
      <div class="item-avatar">${c.type === 'group' ? '👥' : '👤'}</div>
      <div class="item-info">
        <div class="item-name">${c.name}</div>
      </div>
    </div>
  `).join('');
}

// ==========================================
// 初始化
// ==========================================

async function initApp() {
  if (!checkAuth()) return;
  
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
