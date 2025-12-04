# HuanVae Chat 接口调取文档

前端调用示例文档，使用 Fetch API。

## 📂 目录结构

```
接口调取文档/
├── README.md                    # 本文档
├── auth/
│   └── 用户登录注册鉴权部分.md   # 登录、注册、Token 刷新、设备管理
├── profile/
│   └── 个人资料管理.md          # 个人信息、密码修改、头像上传
├── friends/
│   └── 好友添加删除.md          # 好友请求、好友管理
├── messages/
│   └── 好友消息.md              # 好友私聊消息
├── groups/
│   └── 群聊管理.md              # 群聊创建、成员管理、角色管理、邀请码、公告
├── group_messages/
│   └── 群消息.md                # 群聊消息发送、获取、删除、撤回
└── storage/
    └── 文件存储管理.md          # 文件上传、下载、预签名URL
```

## 🔗 快速导航

### 用户认证
- [登录注册鉴权](./auth/用户登录注册鉴权部分.md)
  - 用户注册
  - 用户登录
  - Token 刷新
  - 设备管理

### 个人资料
- [个人资料管理](./profile/个人资料管理.md)
  - 获取个人信息
  - 更新个人信息
  - 修改密码
  - 上传头像

### 好友系统
- [好友添加删除](./friends/好友添加删除.md)
  - 发送好友请求
  - 处理好友请求
  - 好友列表
  - 删除好友

### 私聊消息
- [好友消息](./messages/好友消息.md)
  - 发送消息
  - 获取消息列表
  - 删除消息
  - 撤回消息

### 群聊系统
- [群聊管理](./groups/群聊管理.md)
  - 创建/解散群聊
  - 群设置（修改群名称、上传群头像）
  - 群内昵称管理
  - 成员管理（邀请、移除、退出）
  - 角色管理（群主、管理员）
  - 禁言管理
  - 邀请码管理
  - 入群申请处理
  - 群公告

### 群消息
- [群消息](./group_messages/群消息.md)
  - 发送群消息
  - 获取群消息列表
  - 删除消息（个人）
  - 撤回消息

### 文件存储
- [文件存储管理](./storage/文件存储管理.md)
  - 请求上传（秒传、预签名URL）
  - 预签名PUT直传MinIO
  - 确认上传完成（confirm）
  - UUID 访问 / 预签名 URL
  - 好友文件访问

## 🔧 通用配置

### API 基础地址

```js
// 开发环境（通过 Nginx 代理）
const BASE = 'http://localhost';

// 生产环境
const BASE = 'https://api.huanvae.com';
```

### 通用请求封装

```js
/**
 * 通用 API 请求封装
 * @param {string} path - API 路径
 * @param {object} options - 请求选项
 */
async function api(path, { method='GET', token, body, formData }={}) {
  const headers = {};
  
  // JSON 请求
  if (body) {
    headers['Content-Type'] = 'application/json';
  }
  
  // 认证 Token
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  
  const options = {
    method,
    headers,
    body: formData || (body ? JSON.stringify(body) : undefined)
  };
  
  const res = await fetch(`${BASE}${path}`, options);
  const data = await res.json().catch(() => ({}));
  
  if (!res.ok) {
    throw new Error(`${res.status} ${JSON.stringify(data)}`);
  }
  
  return data;
}
```

### Token 管理

```js
// 存储 Token
function saveTokens(accessToken, refreshToken) {
  localStorage.setItem('access_token', accessToken);
  localStorage.setItem('refresh_token', refreshToken);
}

// 获取 Token
function getToken() {
  return localStorage.getItem('access_token');
}

// 刷新 Token
async function refreshToken() {
  const refreshToken = localStorage.getItem('refresh_token');
  if (!refreshToken) {
    throw new Error('No refresh token');
  }
  
  const res = await fetch(`${BASE}/api/auth/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken })
  });
  
  if (!res.ok) {
    localStorage.clear();
    throw new Error('Token refresh failed');
  }
  
  const data = await res.json();
  saveTokens(data.access_token, data.refresh_token);
  return data.access_token;
}
```

## 📊 API 端点汇总

### 认证相关
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/auth/register` | 用户注册 |
| POST | `/api/auth/login` | 用户登录 |
| POST | `/api/auth/refresh` | 刷新 Token |
| POST | `/api/auth/logout` | 用户登出 |
| GET | `/api/auth/devices` | 获取登录设备列表 |
| DELETE | `/api/auth/devices/{id}` | 删除设备 |

### 个人资料
| 方法 | 端点 | 说明 |
|------|------|------|
| GET | `/api/profile` | 获取个人信息 |
| PUT | `/api/profile` | 更新个人信息 |
| PUT | `/api/profile/password` | 修改密码 |
| POST | `/api/profile/avatar` | 上传头像 |

### 好友系统
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/friends/requests` | 发送好友请求 |
| GET | `/api/friends/requests/pending` | 获取待处理请求 |
| GET | `/api/friends/requests/sent` | 获取发出的请求 |
| POST | `/api/friends/requests/approve` | 同意好友请求 |
| POST | `/api/friends/requests/reject` | 拒绝好友请求 |
| GET | `/api/friends` | 获取好友列表 |
| POST | `/api/friends/remove` | 删除好友 |

### 私聊消息
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/messages` | 发送消息 |
| GET | `/api/messages` | 获取消息列表（`before_time` 时间戳分页） |
| DELETE | `/api/messages/delete` | 删除消息 |
| POST | `/api/messages/recall` | 撤回消息 |

### 群聊系统
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/groups` | 创建群聊 |
| GET | `/api/groups/my` | 获取我的群聊 |
| GET | `/api/groups/search` | 搜索群聊 |
| GET | `/api/groups/{id}` | 获取群详情 |
| PUT | `/api/groups/{id}` | 更新群信息（名称、简介） |
| POST | `/api/groups/{id}/avatar` | 上传群头像 |
| PUT | `/api/groups/{id}/nickname` | 修改我的群内昵称 |
| DELETE | `/api/groups/{id}` | 解散群聊 |
| PUT | `/api/groups/{id}/join-mode` | 修改入群模式 |
| GET | `/api/groups/{id}/members` | 获取成员列表 |
| POST | `/api/groups/{id}/invite` | 邀请成员 |
| POST | `/api/groups/{id}/leave` | 退出群聊 |
| DELETE | `/api/groups/{id}/members/{uid}` | 移除成员 |
| POST | `/api/groups/{id}/transfer` | 转让群主 |
| POST | `/api/groups/{id}/admins` | 设置管理员 |
| DELETE | `/api/groups/{id}/admins/{uid}` | 取消管理员 |
| POST | `/api/groups/{id}/mute` | 禁言成员 |
| DELETE | `/api/groups/{id}/mute/{uid}` | 解除禁言 |
| POST | `/api/groups/{id}/invite-codes` | 生成邀请码 |
| GET | `/api/groups/{id}/invite-codes` | 获取邀请码列表 |
| DELETE | `/api/groups/{id}/invite-codes/{cid}` | 撤销邀请码 |
| POST | `/api/groups/join-by-code` | 通过邀请码入群 |
| POST | `/api/groups/{id}/apply` | 申请入群 |
| GET | `/api/groups/{id}/requests` | 获取待处理申请 |
| POST | `/api/groups/{id}/requests/{rid}/approve` | 同意申请 |
| POST | `/api/groups/{id}/requests/{rid}/reject` | 拒绝申请 |
| GET | `/api/groups/invitations` | 获取收到的邀请 |
| POST | `/api/groups/invitations/{rid}/accept` | 接受邀请 |
| POST | `/api/groups/invitations/{rid}/decline` | 拒绝邀请 |
| POST | `/api/groups/{id}/notices` | 发布公告 |
| GET | `/api/groups/{id}/notices` | 获取公告列表 |
| PUT | `/api/groups/{id}/notices/{nid}` | 更新公告 |
| DELETE | `/api/groups/{id}/notices/{nid}` | 删除公告 |

### 群消息
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/group-messages` | 发送群消息 |
| GET | `/api/group-messages` | 获取群消息列表（`before_time` 时间戳分页，JOIN 优化） |
| DELETE | `/api/group-messages/delete` | 删除消息（个人） |
| POST | `/api/group-messages/recall` | 撤回消息 |

### 文件存储
| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/storage/upload/request` | 请求上传（返回预签名URL） |
| POST | `/api/storage/upload/confirm` | 确认上传完成（预签名上传专用） |
| GET | `/api/storage/file/{uuid}` | UUID 访问文件 |
| POST | `/api/storage/file/{uuid}/presigned-url` | 获取预签名 URL（下载/预览） |
| POST | `/api/storage/file/{uuid}/presigned-url/extended` | 获取扩展预签名 URL（大文件） |
| GET | `/api/storage/files` | 获取文件列表 |
| POST | `/api/storage/friends-file/{uuid}/presigned-url` | 好友文件预签名 URL |

## ⚠️ 注意事项

1. **HTTPS**：生产环境必须使用 HTTPS
2. **Token 过期**：Access Token 有效期 15 分钟，需及时刷新
3. **错误处理**：始终处理 API 调用的错误情况
4. **文件大小限制**：
   - 头像：最大 10MB（用户头像、群头像）
   - 普通文件：<5GB 使用预签名PUT，≥5GB 使用分片上传
5. **消息撤回**：普通用户只能撤回 2 分钟内的消息
6. **实时消息**：已支持 WebSocket 实时推送
7. **消息分页**：使用 `before_time` 时间戳分页
8. **消息归档**：30天前的历史消息会自动归档，仍可正常查询
9. **文件上传**：使用预签名URL直传MinIO，上传后**必须调用confirm接口**确认

## 📝 更新日志

- **2025-12-04**：
  - 消息查询接口新增 `before_time` 时间戳分页参数（性能优化）
  - 群消息查询使用 JOIN 优化，一次性获取发送者信息
  - 新增消息归档功能（30天自动归档）
  - 新增群头像上传、群内昵称修改接口
  - **文件上传改为预签名URL直传MinIO**：
    - 所有文件统一使用预签名URL直传
    - `<5GB` 使用 `presigned_put` 单次PUT上传
    - `≥5GB` 使用 `presigned_multipart` 分片上传
    - 新增 `/api/storage/upload/confirm` 确认上传接口
    - 上传完成后触发WebSocket实时通知
- **2025-12-03**：新增群聊系统和群消息接口文档
- **2025-11-25**：初始版本，包含认证、好友、消息、文件存储接口

