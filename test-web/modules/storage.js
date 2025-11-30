/**
 * 文件存储模块
 */

import { state, saveUploadedFile, getAuthHeaders, cachePresignedUrl, getCachedPresignedUrl } from './state.js';
import { $, pretty, calculateSHA256, formatFileSize, formatTime } from './utils.js';
import { refreshAccessToken } from './auth.js';

// 上传文件（带哈希计算）
export async function uploadFileWithHash() {
  const fileInput = $('file_input');
  const file = fileInput.files[0];
  
  if (!file) {
    $('fileUploadResFmt').textContent = '请先选择文件';
    return;
  }
  
  if (!state.accessToken) {
    $('fileUploadResFmt').textContent = '请先登录';
    return;
  }
  
  try {
    // 显示进度条
    $('uploadProgress').style.display = 'block';
    $('progressText').textContent = '正在计算文件哈希...';
    $('progressBar').style.width = '10%';
    
    // 计算文件哈希
    const fileHash = await calculateSHA256(file);
    console.log('文件哈希:', fileHash);
    
    $('progressText').textContent = '正在请求上传...';
    $('progressBar').style.width = '30%';
    
    // 请求上传
    const uploadRequest = {
      file_type: $('file_type').value,
      storage_location: $('storage_location').value,
      related_id: $('related_id').value.trim() || null,
      filename: file.name,
      file_size: file.size,
      content_type: file.type || 'application/octet-stream',
      file_hash: fileHash,
      force_upload: $('force_upload').checked,
    };
    
    $('fileUploadReqFmt').textContent = pretty(uploadRequest);
    
    const requestRes = await fetch(`${state.storageBase}/upload/request`, {
      method: 'POST',
      headers: getAuthHeaders(),
      body: JSON.stringify(uploadRequest),
    });
    
    const requestData = await requestRes.json();
    $('fileUploadResFmt').textContent = pretty(requestData);
    
    if (!requestRes.ok) {
      throw new Error(requestData.error || '请求上传失败');
    }
    
    // 检查秒传
    if (requestData.instant_upload) {
      $('progressText').textContent = '✅ 秒传成功！';
      $('progressBar').style.width = '100%';
      
      // 保存到已上传文件列表
      saveUploadedFile({
        filename: file.name,
        file_size: file.size,
        file_url: requestData.existing_file_url,
        uploaded_at: new Date().toISOString(),
        instant: true,
      });
      
      renderUploadedFiles();
      
      setTimeout(() => {
        $('uploadProgress').style.display = 'none';
      }, 2000);
      
      return;
    }
    
    // 直接上传
    if (requestData.upload_url) {
      $('progressText').textContent = '正在上传文件...';
      $('progressBar').style.width = '60%';
      
      const formData = new FormData();
      formData.append('file', file);
      
      const uploadRes = await fetch(requestData.upload_url, {
        method: 'POST',
        body: formData,
      });
      
      const uploadData = await uploadRes.json();
      
      if (uploadRes.ok) {
        $('progressText').textContent = '✅ 上传成功！';
        $('progressBar').style.width = '100%';
        $('fileUploadResFmt').textContent = pretty(uploadData);
        
        // 保存到已上传文件列表
        saveUploadedFile({
          filename: file.name,
          file_size: file.size,
          file_url: uploadData.file_url,
          uploaded_at: new Date().toISOString(),
          instant: false,
        });
        
        renderUploadedFiles();
        
        setTimeout(() => {
          $('uploadProgress').style.display = 'none';
          fileInput.value = ''; // 清空文件输入
        }, 2000);
      } else {
        throw new Error(uploadData.error || '上传失败');
      }
    }
    
  } catch (error) {
    console.error('上传失败:', error);
    $('fileUploadResFmt').textContent = `错误: ${error.message}`;
    $('progressText').textContent = '❌ 上传失败';
    $('progressBar').style.width = '0%';
    
    setTimeout(() => {
      $('uploadProgress').style.display = 'none';
    }, 3000);
  }
}

// 渲染已上传文件列表
export function renderUploadedFiles() {
  const container = $('uploadedFilesList');
  if (!container) return;
  
  container.innerHTML = '';
  
  if (state.uploadedFiles.length === 0) {
    container.innerHTML = '<p style="color: #999;">暂无已上传文件</p>';
    return;
  }
  
  state.uploadedFiles.slice().reverse().forEach((file, index) => {
    const div = document.createElement('div');
    div.className = 'file-item';
    div.innerHTML = `
      <div>
        <strong>${file.filename}</strong>
        <span class="muted">${formatFileSize(file.file_size)}</span>
        ${file.instant ? '<span style="color: #4caf50;">（秒传）</span>' : ''}
      </div>
      <div class="muted">${formatTime(file.uploaded_at)}</div>
      <div>
        <button onclick="copyFileUrl('${file.file_url}')">复制URL</button>
        <button onclick="previewFileFromList('${file.file_url}')">预览</button>
      </div>
    `;
    container.appendChild(div);
  });
}

// 复制文件URL
export function copyFileUrl(url) {
  navigator.clipboard.writeText(url).then(() => {
    alert('URL已复制到剪贴板');
  }).catch(() => {
    alert('复制失败，请手动复制');
  });
}

// 从列表预览文件
export function previewFileFromList(url) {
  $('preview_url').value = url;
  previewFile();
}

// 预览文件
export async function previewFile() {
  const url = $('preview_url').value.trim();
  
  if (!url) {
    alert('请输入文件URL');
    return;
  }
  
  const previewDiv = $('filePreview');
  
  try {
    // 从URL提取UUID
    const uuidMatch = url.match(/\/file\/([a-f0-9-]+)/);
    if (!uuidMatch) {
      throw new Error('无效的文件URL');
    }
    
    const uuid = uuidMatch[1];
    
    // 检查缓存的预签名URL
    let presignedUrl = getCachedPresignedUrl(uuid);
    
    if (!presignedUrl) {
      // 获取新的预签名URL
      previewDiv.innerHTML = '<p style="color: #999;">正在获取文件...</p>';
      
      const response = await fetch(url.replace(/\/file\/[^\/]+$/, `/file/${uuid}/presigned-url`), {
        method: 'POST',
        headers: getAuthHeaders(),
        body: JSON.stringify({ operation: 'download' }),
      });
      
      if (!response.ok) {
        if (response.status === 401) {
          // Token过期，刷新后重试
          await refreshAccessToken();
          return previewFile();
        }
        throw new Error(`获取失败: ${response.status}`);
      }
      
      const data = await response.json();
      presignedUrl = data.presigned_url;
      
      // 缓存URL
      cachePresignedUrl(uuid, presignedUrl, data.expires_at);
    }
    
    // 根据文件类型显示预览
    const contentType = await getContentType(presignedUrl);
    
    if (contentType.startsWith('image/')) {
      previewDiv.innerHTML = `<img src="${presignedUrl}" style="max-width: 100%; max-height: 500px;" alt="图片预览" />`;
    } else if (contentType.startsWith('video/')) {
      previewDiv.innerHTML = `
        <video controls style="max-width: 100%; max-height: 500px;">
          <source src="${presignedUrl}" type="${contentType}" />
          您的浏览器不支持视频播放
        </video>
      `;
    } else {
      previewDiv.innerHTML = `
        <p>文件类型: ${contentType}</p>
        <p><a href="${presignedUrl}" target="_blank" download>点击下载文件</a></p>
      `;
    }
    
  } catch (error) {
    console.error('预览失败:', error);
    previewDiv.innerHTML = `<p style="color: #f44336;">预览失败: ${error.message}</p>`;
  }
}

// 获取文件Content-Type
async function getContentType(url) {
  try {
    const response = await fetch(url, { method: 'HEAD' });
    return response.headers.get('Content-Type') || 'application/octet-stream';
  } catch {
    return 'application/octet-stream';
  }
}

// ⭐ 新增：查询个人文件列表
export async function listUserFiles(page = 1, limit = 20, sortBy = 'created_at', sortOrder = 'desc') {
  if (!state.accessToken) {
    $('fileListResFmt').textContent = '请先登录';
    return;
  }
  
  try {
    const params = new URLSearchParams({
      page: page.toString(),
      limit: limit.toString(),
      sort_by: sortBy,
      sort_order: sortOrder,
    });
    
    const response = await fetch(`${state.storageBase}/files?${params}`, {
      method: 'GET',
      headers: getAuthHeaders(),
    });
    
    if (!response.ok) {
      if (response.status === 401) {
        // Token过期，刷新后重试
        await refreshAccessToken();
        return listUserFiles(page, limit, sortBy, sortOrder);
      }
      throw new Error(`查询失败: ${response.status}`);
    }
    
    const data = await response.json();
    $('fileListResFmt').textContent = pretty(data);
    
    // 渲染文件列表
    renderFileList(data);
    
    return data;
  } catch (error) {
    console.error('查询文件列表失败:', error);
    $('fileListResFmt').textContent = `错误: ${error.message}`;
    throw error;
  }
}

// 渲染文件列表
function renderFileList(data) {
  const container = $('fileListDisplay');
  if (!container) return;
  
  container.innerHTML = '';
  
  if (!data.files || data.files.length === 0) {
    container.innerHTML = '<p style="color: #999;">暂无文件</p>';
    return;
  }
  
  // 文件列表
  data.files.forEach(file => {
    const div = document.createElement('div');
    div.className = 'file-item';
    div.innerHTML = `
      <div>
        <strong>${file.filename}</strong>
        <span class="muted">${formatFileSize(file.file_size)}</span>
        <span class="muted">${file.preview_support === 'inline_preview' ? '可预览' : '仅下载'}</span>
      </div>
      <div class="muted">
        上传时间: ${formatTime(file.created_at)}
      </div>
      <div class="muted">
        UUID: ${file.file_uuid}
      </div>
      <div>
        <button onclick="previewFileFromList('${file.file_url}')">预览</button>
        <button onclick="copyFileUrl('${file.file_url}')">复制URL</button>
      </div>
    `;
    container.appendChild(div);
  });
  
  // 分页信息
  const paginationDiv = document.createElement('div');
  paginationDiv.style.marginTop = '20px';
  paginationDiv.style.textAlign = 'center';
  paginationDiv.innerHTML = `
    <div>
      <span>共 ${data.total} 个文件，第 ${data.page}/${data.total_pages} 页</span>
    </div>
    <div style="margin-top: 10px;">
      <button ${data.page <= 1 ? 'disabled' : ''} onclick="loadFilePage(${data.page - 1})">上一页</button>
      <button ${!data.has_more ? 'disabled' : ''} onclick="loadFilePage(${data.page + 1})">下一页</button>
    </div>
  `;
  container.appendChild(paginationDiv);
}

// 加载指定页
export function loadFilePage(page) {
  const limit = parseInt($('fileListLimit').value) || 20;
  const sortBy = $('fileListSortBy').value || 'created_at';
  const sortOrder = $('fileListSortOrder').value || 'desc';
  listUserFiles(page, limit, sortBy, sortOrder);
}

// 挂载到window供HTML调用
if (typeof window !== 'undefined') {
  window.copyFileUrl = copyFileUrl;
  window.previewFileFromList = previewFileFromList;
  window.loadFilePage = loadFilePage;
}

