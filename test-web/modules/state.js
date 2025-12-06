/**
 * 状态管理模块
 */

// 根据当前域名自动选择 API 地址
const BASE_HOST = window.location.hostname.startsWith('web.') 
  ? `${window.location.protocol}//api.${window.location.hostname.slice(4)}`  // web.xxx -> api.xxx
  : `${window.location.protocol}//${window.location.hostname}`;  // 本地开发使用当前地址

export const state = {
  apiBase: localStorage.getItem('apiBase') || `${BASE_HOST}/api/auth`,
  friendsBase: localStorage.getItem('friendsApiBase') || `${BASE_HOST}/api/friends`,
  messagesBase: localStorage.getItem('messagesApiBase') || `${BASE_HOST}/api/messages`,
  storageBase: localStorage.getItem('storageApiBase') || `${BASE_HOST}/api/storage`,
  accessToken: localStorage.getItem('accessToken') || '',
  refreshToken: localStorage.getItem('refreshToken') || '',
  uploadedFiles: JSON.parse(localStorage.getItem('uploadedFiles') || '[]'),
  presignedUrls: {}, // 缓存预签名URL: { uuid: { url, expiresAt } }
};

// 保存 Token 到状态和 localStorage
export function saveTokens(accessToken, refreshToken) {
  state.accessToken = accessToken;
  state.refreshToken = refreshToken;
  localStorage.setItem('accessToken', accessToken);
  localStorage.setItem('refreshToken', refreshToken);
}

// 清空 Token
export function clearTokens() {
  state.accessToken = '';
  state.refreshToken = '';
  localStorage.removeItem('accessToken');
  localStorage.removeItem('refreshToken');
}

// 保存 API 基地址
export function saveApiBase(key, value) {
  state[key] = value;
  const storageKey = key === 'apiBase' ? 'apiBase' :
                     key === 'friendsBase' ? 'friendsApiBase' :
                     key === 'messagesBase' ? 'messagesApiBase' :
                     'storageApiBase';
  localStorage.setItem(storageKey, value);
}

// 保存已上传文件
export function saveUploadedFile(fileInfo) {
  state.uploadedFiles.push(fileInfo);
  localStorage.setItem('uploadedFiles', JSON.stringify(state.uploadedFiles));
}

// 获取请求头（带 Token）
export function getAuthHeaders() {
  return {
    'Authorization': `Bearer ${state.accessToken}`,
    'Content-Type': 'application/json'
  };
}

// 解码 JWT Token
export function decodeJwt(token) {
  try {
    const [, payload] = token.split('.');
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(json);
  } catch {
    return null;
  }
}

// 从 Token 获取 claims
export function claimsFromToken() {
  const c = decodeJwt(state.accessToken);
  return c || {};
}

// 缓存预签名 URL
export function cachePresignedUrl(uuid, url, expiresAt) {
  state.presignedUrls[uuid] = { url, expiresAt, cachedAt: new Date().toISOString() };
}

// 获取缓存的预签名 URL
export function getCachedPresignedUrl(uuid) {
  const cached = state.presignedUrls[uuid];
  if (!cached) return null;
  
  const expiresTime = new Date(cached.expiresAt);
  const now = new Date();
  const remainingMs = expiresTime - now;
  
  // 过期前5分钟需要刷新
  if (remainingMs > 5 * 60 * 1000) {
    return cached.url;
  }
  
  // 已过期或即将过期
  delete state.presignedUrls[uuid];
  return null;
}

