// Service Worker - 用于代理需要Authorization的媒体请求
// 让video标签可以直接使用UUID URL进行流式播放

const CACHE_NAME = 'media-auth-cache-v1';
const AUTH_URLS = ['/api/storage/file/'];

// Token存储（通过postMessage从主线程接收）
let accessToken = '';

// 从存储中获取token
function getAccessToken() {
  return accessToken || '';
}

// 安装事件
self.addEventListener('install', (event) => {
  console.log('[SW] Service Worker 安装中...');
  // 立即激活新的Service Worker
  self.skipWaiting();
});

// 激活事件
self.addEventListener('activate', (event) => {
  console.log('[SW] Service Worker 已激活');
  // 清理旧缓存
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          if (cacheName !== CACHE_NAME) {
            console.log('[SW] 删除旧缓存:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    })
  );
  // 立即控制所有页面
  return self.clients.claim();
});

// 接收来自主线程的消息（用于更新token）
self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SET_TOKEN') {
    accessToken = event.data.token;
    console.log('[SW] Access Token 已更新:', accessToken ? '有token' : '无token');
  }
});

// 拦截fetch请求
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  
  // 检查是否是需要代理的URL
  const needsAuth = AUTH_URLS.some(authUrl => url.pathname.startsWith(authUrl));
  
  if (needsAuth) {
    console.log('[SW] 拦截请求:', url.pathname);
    event.respondWith(handleAuthenticatedRequest(event.request));
  } else {
    // 其他请求正常处理
    event.respondWith(fetch(event.request));
  }
});

// 处理需要Authorization的请求
async function handleAuthenticatedRequest(request) {
  const token = getAccessToken();
  
  if (!token) {
    console.error('[SW] 缺少 Access Token');
    return new Response('Unauthorized: No access token', { 
      status: 401,
      statusText: 'Unauthorized'
    });
  }
  
  try {
    // 克隆请求并添加Authorization头
    const authenticatedRequest = new Request(request, {
      headers: new Headers({
        ...Object.fromEntries(request.headers.entries()),
        'Authorization': `Bearer ${token}`
      })
    });
    
    console.log('[SW] 发送带Authorization的请求');
    
    // 发送请求
    const response = await fetch(authenticatedRequest);
    
    // 检查响应状态
    if (!response.ok) {
      console.error('[SW] 请求失败:', response.status, response.statusText);
      return response;
    }
    
    console.log('[SW] 请求成功:', response.status);
    
    // 对于Range请求，确保正确处理
    if (request.headers.get('Range')) {
      console.log('[SW] Range请求:', request.headers.get('Range'));
    }
    
    // 克隆响应以便缓存（可选）
    const responseToCache = response.clone();
    
    // 可选：缓存小文件的响应
    const contentLength = response.headers.get('content-length');
    if (contentLength && parseInt(contentLength) < 10 * 1024 * 1024) {
      // 小于10MB的文件可以缓存
      caches.open(CACHE_NAME).then(cache => {
        cache.put(request, responseToCache);
      });
    }
    
    return response;
    
  } catch (error) {
    console.error('[SW] 请求异常:', error);
    return new Response('Network error', { 
      status: 503,
      statusText: 'Service Unavailable'
    });
  }
}

// 错误处理
self.addEventListener('error', (event) => {
  console.error('[SW] Service Worker 错误:', event.error);
});

self.addEventListener('unhandledrejection', (event) => {
  console.error('[SW] 未处理的Promise拒绝:', event.reason);
});

console.log('[SW] Service Worker 脚本已加载');

