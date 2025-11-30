/**
 * UI渲染模块
 */

import { state } from './state.js';
import { $, pretty, decodeJwt } from './utils.js';

// 渲染本地状态
export function renderLocalState() {
  if (!$('localState')) return;
  
  const claims = state.accessToken ? decodeJwt(state.accessToken) : null;
  const info = {
    hasAccessToken: !!state.accessToken,
    hasRefreshToken: !!state.refreshToken,
    tokenExpiry: claims ? new Date(claims.exp * 1000).toISOString() : 'N/A',
    user_id: claims?.sub || 'N/A',
    uploadedFilesCount: state.uploadedFiles.length,
  };
  $('localState').textContent = pretty(info);
}

// 初始化UI
export function initUI() {
  // 设置API基地址
  if ($('apiBase')) $('apiBase').value = state.apiBase;
  if ($('friendsApiBase')) $('friendsApiBase').value = state.friendsBase;
  if ($('messagesApiBase')) $('messagesApiBase').value = state.messagesBase;
  if ($('storageApiBase')) $('storageApiBase').value = state.storageBase;
  
  // 渲染本地状态
  renderLocalState();
}

// 通用展示请求信息
export function showRequest(id, req) {
  const elem = $(id);
  if (!elem) return;
  
  const { method, url, headers, body } = req;
  const out = { method, url, headers, body };
  elem.textContent = pretty(out);
}

// 通用JSON请求封装
export async function doJson(req, outId) {
  try {
    if (outId) showRequest(outId, req);
    
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

