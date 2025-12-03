/**
 * 好友功能模块（简化版）
 */

import { state, getAuthHeaders, claimsFromToken } from './state.js';
import { $, pretty } from './utils.js';

// 提交好友请求
export async function submitFriendRequest() {
  if (!state.accessToken) {
    $('submitResFmt').textContent = '未登录';
    return;
  }
  
  // 验证 Token 是否有效
  const claims = claimsFromToken();
  if (!claims || !claims.sub) {
    $('submitResFmt').textContent = 'Token 无效或已过期，请重新登录';
    return;
  }
  
  // 验证目标用户 ID
  const targetUserId = $('req_target_user_id').value.trim();
  if (!targetUserId) {
    $('submitResFmt').textContent = '请输入目标用户ID';
    return;
  }
  
  const body = {
    user_id: claims.sub,  // 后端需要这个字段
    target_user_id: targetUserId,
    reason: $('req_reason').value.trim() || undefined,  // 修正字段名
    request_time: $('req_request_time').value.trim() || new Date().toISOString(),
  };
  
  try {
    const res = await fetch(`${state.friendsBase}/requests`, {
      method: 'POST',
      headers: getAuthHeaders(),
      body: JSON.stringify(body),
    });
    
    // 处理可能的非 JSON 响应
    const contentType = res.headers.get('content-type') || '';
    let data;
    if (contentType.includes('application/json')) {
      data = await res.json();
    } else {
      const text = await res.text();
      data = { error: text };
    }
    
    $('submitResFmt').textContent = pretty(data);
  } catch (err) {
    $('submitResFmt').textContent = String(err);
  }
}

// 获取好友列表
export async function listFriends() {
  if (!state.accessToken) {
    $('friendsResFmt').textContent = '未登录';
    return;
  }
  
  try {
    const res = await fetch(`${state.friendsBase}`, {
      method: 'GET',
      headers: getAuthHeaders(),
    });
    const data = await res.json();
    $('friendsResFmt').textContent = pretty(data);
    
    // 渲染好友列表
    const container = $('friendsList');
    if (container) {
      container.innerHTML = '';
      if (data.items && Array.isArray(data.items)) {
        data.items.forEach(friend => {
          const div = document.createElement('div');
          div.className = 'friend-item';
          div.innerHTML = `
            <strong>${friend.friend_id}</strong>
            <span class="muted">${friend.friend_nickname || ''}</span>
          `;
          container.appendChild(div);
        });
      }
    }
  } catch (err) {
    $('friendsResFmt').textContent = String(err);
  }
}

// 更多好友功能可以根据需要添加...

