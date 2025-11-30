/**
 * 主入口文件 - 模块化版本
 */

import { $, pretty } from './modules/utils.js';
import { state, saveApiBase, clearTokens } from './modules/state.js';
import { register, login, refreshAccessToken, showProfileFromAccessToken, listDevices } from './modules/auth.js';
import { submitFriendRequest, listFriends } from './modules/friends.js';
import { sendMessage, getMessages } from './modules/messages.js';
import { uploadFileWithHash, previewFile, renderUploadedFiles, listUserFiles } from './modules/storage.js';
import { renderLocalState, initUI } from './modules/ui.js';

// 初始化应用
function init() {
  console.log('📦 应用启动（模块化版本）');
  
  // 初始化UI
  initUI();
  
  // 后端设置保存按钮
  $('saveBase').onclick = () => {
    saveApiBase('apiBase', $('apiBase').value.trim() || state.apiBase);
    renderLocalState();
  };
  
  $('saveFriendsBase').onclick = () => {
    saveApiBase('friendsBase', $('friendsApiBase').value.trim() || state.friendsBase);
    renderLocalState();
  };
  
  $('saveMessagesBase').onclick = () => {
    saveApiBase('messagesBase', $('messagesApiBase').value.trim() || state.messagesBase);
    renderLocalState();
  };
  
  $('saveStorageBase').onclick = () => {
    saveApiBase('storageBase', $('storageApiBase').value.trim() || state.storageBase);
    renderLocalState();
  };
  
  // 认证功能
  $('btnRegister').onclick = register;
  $('btnLogin').onclick = login;
  $('btnShowProfile').onclick = showProfileFromAccessToken;
  $('btnRefreshToken').onclick = async () => {
    try {
      await refreshAccessToken();
      renderLocalState();
    } catch (error) {
      console.error('刷新Token失败:', error);
    }
  };
  $('btnListDevices').onclick = listDevices;
  $('btnClear').onclick = () => {
    clearTokens();
    renderLocalState();
    $('profile').textContent = '';
    $('devices').innerHTML = '';
  };
  
  // 好友功能
  $('btnSubmitFriend').onclick = submitFriendRequest;
  $('btnListFriends').onclick = listFriends;
  
  // 消息功能
  $('btnSendMessage').onclick = sendMessage;
  $('btnGetMessages').onclick = getMessages;
  
  // 文件存储功能
  $('btnUploadFile').onclick = uploadFileWithHash;
  $('btnPreviewFile').onclick = previewFile;
  renderUploadedFiles();
  
  // ⭐ 新增：文件列表查询功能
  if ($('btnListFiles')) {
    $('btnListFiles').onclick = () => {
      const page = parseInt($('fileListPage').value) || 1;
      const limit = parseInt($('fileListLimit').value) || 20;
      const sortBy = $('fileListSortBy').value || 'created_at';
      const sortOrder = $('fileListSortOrder').value || 'desc';
      listUserFiles(page, limit, sortBy, sortOrder);
    };
  }
  
  console.log('✅ 应用初始化完成');
}

// 页面加载完成后初始化
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

