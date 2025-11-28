// Token Manager - 专门管理token和Service Worker通信

class TokenManager {
  constructor() {
    this.accessToken = localStorage.getItem('accessToken') || '';
    this.refreshToken = localStorage.getItem('refreshToken') || '';
    this.serviceWorkerReady = false;
    this.listeners = [];
  }

  // 初始化Token Manager
  async initialize() {
    console.log('[TokenManager] 初始化...');
    
    // 监听localStorage变化（跨标签页同步）
    window.addEventListener('storage', (e) => {
      if (e.key === 'accessToken') {
        this.accessToken = e.newValue || '';
        console.log('[TokenManager] 检测到token变化（其他标签页）');
        this.syncToServiceWorker();
        this.notifyListeners();
      }
    });

    // 注册Service Worker
    await this.registerServiceWorker();
  }

  // 注册Service Worker
  async registerServiceWorker() {
    if (!('serviceWorker' in navigator)) {
      console.warn('[TokenManager] 浏览器不支持 Service Worker');
      return false;
    }

    try {
      // 先尝试获取现有注册
      const existingRegistration = await navigator.serviceWorker.getRegistration();
      
      if (existingRegistration) {
        console.log('[TokenManager] 发现已存在的 Service Worker');
        await navigator.serviceWorker.ready;
        
        // 如果已经有controller，立即同步token
        if (navigator.serviceWorker.controller) {
          console.log('[TokenManager] Service Worker 已控制页面');
          this.serviceWorkerReady = true;
          await this.syncToServiceWorker();
          return true;
        }
      }

      // 注册新的Service Worker
      console.log('[TokenManager] 注册 Service Worker...');
      const registration = await navigator.serviceWorker.register('./sw.js', {
        scope: '/'
      });

      console.log('[TokenManager] Service Worker 注册成功:', registration.scope);

      // 等待激活
      await navigator.serviceWorker.ready;
      
      // 检查是否需要刷新页面
      if (!navigator.serviceWorker.controller) {
        console.warn('[TokenManager] ⚠️ Service Worker 未控制页面，需要刷新');
        this.serviceWorkerReady = false;
        return false;
      }

      this.serviceWorkerReady = true;
      console.log('[TokenManager] ✅ Service Worker 已就绪');
      
      // 立即同步token
      await this.syncToServiceWorker();
      
      return true;
    } catch (error) {
      console.error('[TokenManager] Service Worker 注册失败:', error);
      return false;
    }
  }

  // 同步token到Service Worker
  async syncToServiceWorker() {
    if (!navigator.serviceWorker.controller) {
      console.warn('[TokenManager] ⚠️ Service Worker 控制器不存在，无法同步token');
      return false;
    }

    if (!this.accessToken) {
      console.warn('[TokenManager] ⚠️ 没有可用的 access token');
      return false;
    }

    try {
      navigator.serviceWorker.controller.postMessage({
        type: 'SET_TOKEN',
        token: this.accessToken
      });
      console.log('[TokenManager] 📤 已发送 token 到 Service Worker');
      return true;
    } catch (error) {
      console.error('[TokenManager] 发送token失败:', error);
      return false;
    }
  }

  // 设置token（登录时调用）
  async setTokens(accessToken, refreshToken) {
    console.log('[TokenManager] 设置新的tokens');
    
    this.accessToken = accessToken;
    this.refreshToken = refreshToken;
    
    // 保存到localStorage
    localStorage.setItem('accessToken', accessToken);
    localStorage.setItem('refreshToken', refreshToken);
    
    // 同步到Service Worker
    await this.syncToServiceWorker();
    
    // 通知监听器
    this.notifyListeners();
  }

  // 更新access token（刷新token时调用）
  async updateAccessToken(accessToken) {
    console.log('[TokenManager] 更新 access token');
    
    this.accessToken = accessToken;
    localStorage.setItem('accessToken', accessToken);
    
    // 同步到Service Worker
    await this.syncToServiceWorker();
    
    // 通知监听器
    this.notifyListeners();
  }

  // 清除tokens（登出时调用）
  clearTokens() {
    console.log('[TokenManager] 清除 tokens');
    
    this.accessToken = '';
    this.refreshToken = '';
    
    localStorage.removeItem('accessToken');
    localStorage.removeItem('refreshToken');
    
    // 通知Service Worker
    if (navigator.serviceWorker.controller) {
      navigator.serviceWorker.controller.postMessage({
        type: 'SET_TOKEN',
        token: ''
      });
    }
    
    // 通知监听器
    this.notifyListeners();
  }

  // 获取access token
  getAccessToken() {
    return this.accessToken;
  }

  // 获取refresh token
  getRefreshToken() {
    return this.refreshToken;
  }

  // 检查是否有token
  hasToken() {
    return !!this.accessToken;
  }

  // 检查Service Worker是否就绪
  isServiceWorkerReady() {
    return this.serviceWorkerReady && !!navigator.serviceWorker.controller;
  }

  // 添加token变化监听器
  addListener(callback) {
    this.listeners.push(callback);
  }

  // 移除监听器
  removeListener(callback) {
    this.listeners = this.listeners.filter(l => l !== callback);
  }

  // 通知所有监听器
  notifyListeners() {
    this.listeners.forEach(callback => {
      try {
        callback(this.accessToken, this.refreshToken);
      } catch (error) {
        console.error('[TokenManager] 监听器错误:', error);
      }
    });
  }

  // 强制刷新页面以启用Service Worker
  reloadForServiceWorker() {
    console.log('[TokenManager] 刷新页面以启用 Service Worker...');
    window.location.reload();
  }

  // 获取状态信息（用于调试）
  getStatus() {
    return {
      hasAccessToken: !!this.accessToken,
      hasRefreshToken: !!this.refreshToken,
      serviceWorkerSupported: 'serviceWorker' in navigator,
      serviceWorkerReady: this.serviceWorkerReady,
      serviceWorkerController: !!navigator.serviceWorker.controller,
      accessTokenLength: this.accessToken.length,
    };
  }

  // 诊断工具
  async diagnose() {
    console.log('=== Token Manager 诊断信息 ===');
    const status = this.getStatus();
    
    console.log('1. 浏览器支持 SW:', status.serviceWorkerSupported ? '✅' : '❌');
    console.log('2. SW已就绪:', status.serviceWorkerReady ? '✅' : '❌');
    console.log('3. SW控制器存在:', status.serviceWorkerController ? '✅' : '❌');
    console.log('4. 有Access Token:', status.hasAccessToken ? '✅' : '❌');
    console.log('5. Token长度:', status.accessTokenLength);
    
    if (status.hasAccessToken && status.serviceWorkerController) {
      console.log('6. 尝试同步token...');
      const success = await this.syncToServiceWorker();
      console.log('   同步结果:', success ? '✅ 成功' : '❌ 失败');
    }
    
    if (!status.serviceWorkerController) {
      console.warn('⚠️ 建议：刷新页面以启用 Service Worker');
    }
    
    return status;
  }
}

// 创建全局实例
const tokenManager = new TokenManager();

// 导出
window.tokenManager = tokenManager;

console.log('[TokenManager] 模块已加载');

