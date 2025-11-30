/**
 * 工具函数模块
 */

// DOM 选择器简化
export const $ = (id) => document.getElementById(id);

// JSON 格式化
export const pretty = (obj) => JSON.stringify(obj, null, 2);

// 格式化文件大小
export function formatFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(2) + ' MB';
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}

// 格式化时间
export function formatTime(isoString) {
  try {
    return new Date(isoString).toLocaleString('zh-CN');
  } catch {
    return isoString;
  }
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

// 计算文件采样 SHA-256 哈希
export async function calculateSHA256(file) {
  const SAMPLE_SIZE = 10 * 1024 * 1024; // 10MB
  
  // 文件元信息
  const metadata = `${file.name}|${file.size}|${file.lastModified}|${file.type}`;
  const metadataBuffer = new TextEncoder().encode(metadata);
  
  let dataToHash;
  
  if (file.size <= SAMPLE_SIZE * 3) {
    // 小文件（< 30MB）：完整哈希
    dataToHash = await file.arrayBuffer();
  } else {
    // 大文件：采样哈希
    const chunks = [];
    
    // 读取开头10MB
    const startBlob = file.slice(0, SAMPLE_SIZE);
    chunks.push(new Uint8Array(await startBlob.arrayBuffer()));
    
    // 读取中间10MB
    const middleStart = Math.floor((file.size - SAMPLE_SIZE) / 2);
    const middleBlob = file.slice(middleStart, middleStart + SAMPLE_SIZE);
    chunks.push(new Uint8Array(await middleBlob.arrayBuffer()));
    
    // 读取结尾10MB
    const endBlob = file.slice(file.size - SAMPLE_SIZE, file.size);
    chunks.push(new Uint8Array(await endBlob.arrayBuffer()));
    
    // 合并所有数据
    const totalLength = metadataBuffer.length + chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    dataToHash = new Uint8Array(totalLength);
    let offset = 0;
    
    dataToHash.set(metadataBuffer, offset);
    offset += metadataBuffer.length;
    
    for (const chunk of chunks) {
      dataToHash.set(chunk, offset);
      offset += chunk.length;
    }
  }
  
  // 计算SHA-256哈希
  const hashBuffer = await crypto.subtle.digest('SHA-256', dataToHash);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

// 延迟函数
export const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));

