/**
 * 认证模块
 */

import { state, saveTokens, getAuthHeaders } from './state.js';
import { $, pretty, decodeJwt } from './utils.js';

// 注册
export async function register() {
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
    return data;
  } catch (err) {
    $('regResult').textContent = String(err);
    throw err;
  }
}

// 登录
export async function login() {
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
      saveTokens(data.access_token, data.refresh_token);
    }
    return data;
  } catch (err) {
    $('loginResult').textContent = String(err);
    throw err;
  }
}

// 刷新Token
export async function refreshAccessToken() {
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
      saveTokens(data.access_token, state.refreshToken);
      if ($('profile')) $('profile').textContent = '✅ Access Token已刷新';
      console.log('✅ Access Token刷新成功');
      return data;
    } else {
      const error = data.error || 'Token刷新失败';
      console.error('❌ Token刷新失败:', error);
      if ($('profile')) $('profile').textContent = pretty(data);
      throw new Error(error);
    }
  } catch (err) {
    console.error('❌ Token刷新异常:', String(err));
    if ($('profile')) $('profile').textContent = String(err);
    throw err;
  }
}

// 显示个人信息（从Token解析）
export function showProfileFromAccessToken() {
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

// 获取设备列表
export async function listDevices() {
  if (!state.accessToken) {
    $('deviceResult').textContent = '未登录';
    return;
  }

  try {
    const res = await fetch(`${state.apiBase}/devices`, {
      method: 'GET',
      headers: getAuthHeaders(),
    });
    const data = await res.json();
    $('deviceResult').textContent = pretty(data);
    
    // 渲染设备列表
    const devicesDiv = $('devices');
    devicesDiv.innerHTML = '';
    
    if (data.devices && Array.isArray(data.devices)) {
      data.devices.forEach(device => {
        const div = document.createElement('div');
        div.className = 'device-item';
        div.innerHTML = `
          <div><strong>${device.device_info || 'Unknown'}</strong> - ${device.device_id}</div>
          <div class="muted">最后活跃: ${new Date(device.last_used).toLocaleString()}</div>
          <button onclick="revokeDevice('${device.device_id}')">撤销</button>
        `;
        devicesDiv.appendChild(div);
      });
    }
    
    return data;
  } catch (err) {
    $('deviceResult').textContent = String(err);
    throw err;
  }
}

// 撤销设备
export async function revokeDevice(deviceId) {
  if (!state.accessToken) {
    alert('未登录');
    return;
  }

  try {
    const res = await fetch(`${state.apiBase}/devices/${deviceId}`, {
      method: 'DELETE',
      headers: getAuthHeaders(),
    });
    const data = await res.json();
    $('deviceResult').textContent = pretty(data);
    if (res.ok) {
      await listDevices(); // 刷新设备列表
    }
    return data;
  } catch (err) {
    $('deviceResult').textContent = String(err);
    throw err;
  }
}

// 挂载到window供HTML调用
if (typeof window !== 'undefined') {
  window.revokeDevice = revokeDevice;
}

