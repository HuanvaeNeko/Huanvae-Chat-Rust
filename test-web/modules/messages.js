/**
 * 消息功能模块（简化版）
 */

import { state, getAuthHeaders } from './state.js';
import { $, pretty, formatTime } from './utils.js';

// 发送消息
export async function sendMessage() {
  if (!state.accessToken) {
    $('msgSendResFmt').textContent = '未登录';
    return;
  }
  
  const body = {
    receiver_id: $('msg_receiver_id').value.trim(),
    message_type: $('msg_type').value,
    message_content: $('msg_content').value.trim(),
    file_url: $('msg_file_url').value.trim() || undefined,
    file_size: parseInt($('msg_file_size').value) || undefined,
  };
  
  try {
    const res = await fetch(`${state.messagesBase}`, {
      method: 'POST',
      headers: getAuthHeaders(),
      body: JSON.stringify(body),
    });
    const data = await res.json();
    $('msgSendResFmt').textContent = pretty(data);
  } catch (err) {
    $('msgSendResFmt').textContent = String(err);
  }
}

// 获取消息列表
export async function getMessages() {
  if (!state.accessToken) {
    $('msgGetResFmt').textContent = '未登录';
    return;
  }
  
  const friendId = $('msg_friend_id').value.trim();
  if (!friendId) {
    $('msgGetResFmt').textContent = '请输入好友ID';
    return;
  }
  
  const params = new URLSearchParams({
    friend_id: friendId,
    limit: $('msg_limit').value || '50',
  });
  
  const beforeUuid = $('msg_before_uuid').value.trim();
  if (beforeUuid) {
    params.append('before_uuid', beforeUuid);
  }
  
  try {
    const res = await fetch(`${state.messagesBase}?${params}`, {
      method: 'GET',
      headers: getAuthHeaders(),
    });
    const data = await res.json();
    $('msgGetResFmt').textContent = pretty(data);
    
    // 渲染消息列表
    const container = $('messagesList');
    if (container && data.messages && Array.isArray(data.messages)) {
      container.innerHTML = '';
      data.messages.forEach(msg => {
        const div = document.createElement('div');
        div.className = 'message-item';
        div.innerHTML = `
          <div><strong>${msg.sender_id}</strong> → <strong>${msg.receiver_id}</strong></div>
          <div>${msg.message_content}</div>
          <div class="muted">${formatTime(msg.send_time)}</div>
        `;
        container.appendChild(div);
      });
    }
  } catch (err) {
    $('msgGetResFmt').textContent = String(err);
  }
}

// 更多消息功能可以根据需要添加...

