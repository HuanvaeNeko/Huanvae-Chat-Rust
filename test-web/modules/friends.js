/**
 * 好友功能模块（简化版）
 */

import { state, getAuthHeaders } from './state.js';
import { $, pretty } from './utils.js';

// 提交好友请求
export async function submitFriendRequest() {
  if (!state.accessToken) {
    $('submitResFmt').textContent = '未登录';
    return;
  }
  
  const body = {
    target_user_id: $('req_target_user_id').value.trim(),
    request_message: $('req_reason').value.trim() || undefined,
    request_time: $('req_request_time').value.trim() || new Date().toISOString(),
  };
  
  try {
    const res = await fetch(`${state.friendsBase}/requests`, {
      method: 'POST',
      headers: getAuthHeaders(),
      body: JSON.stringify(body),
    });
    const data = await res.json();
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

