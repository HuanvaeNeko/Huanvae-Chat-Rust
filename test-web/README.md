# HuanVae Chat - Web 客户端

一个现代化的即时通讯 Web 客户端，采用类似微信/QQ 的三栏布局设计。

## ✨ 功能特性

### 🔐 认证系统
- 用户注册与登录
- JWT Token 自动刷新
- 多设备管理
- 安全登出

### 👥 好友系统
- 发送/接收好友请求
- 好友列表管理
- 好友删除

### 💬 私聊消息
- 文本消息发送
- 图片/视频/文件发送
- 消息撤回（2分钟内）
- 消息历史查看（按时间正序，旧消息在上，新消息在下）
- 文件预览与下载
- 好友头像显示

### 👨‍👩‍👧‍👦 群聊系统
- 创建群聊（单人可创建）
- 群设置（群主/管理员）
  - 修改群名称
  - 上传群头像
- 群内昵称（群成员）
  - 每个成员可设置在该群的显示昵称
- 群成员管理
  - 邀请好友加入
  - 设置/取消管理员
  - 踢出成员
- 群公告发布与查看
- 邀请码生成与使用
- 群主转让
- 退出/解散群聊
- 入群方式设置
  - 开放加入
  - 需要审批
  - 仅邀请
  - 仅管理员邀请
  - 禁止加入

### 📁 文件存储
- 个人文件上传与管理
- 秒传功能（SHA-256 去重）
- 大文件采样哈希
- 预签名 URL 下载
- 文件列表与预览
- **文件分类**: 个人文件列表仅显示个人文件，好友/群聊文件单独管理

### 📡 WebSocket 实时通信
- **实时消息推送** - 新消息即时通知
- **未读消息摘要** - 登录时获取所有未读消息统计
- **已读同步** - 标记已读后自动同步
- **消息撤回通知** - 实时接收撤回事件
- **系统通知** - 好友请求、群邀请等系统事件
- **连接状态指示** - 左下角显示实时连接状态
- **自动重连** - 断线后自动尝试重连
- **浏览器通知** - 支持桌面通知（需授权）

### 👤 个人资料
- 头像上传（最大 10MB）
- 个人信息修改
- 密码修改

## 🎨 界面设计

采用现代化三栏布局：

```
┌──────┬────────────┬──────────────────────┬────────────┐
│      │            │                      │            │
│ 导航 │  会话列表   │      聊天区域         │  信息面板  │
│  栏  │            │                      │            │
│      │            │                      │            │
└──────┴────────────┴──────────────────────┴────────────┘
```

### 导航栏
- 用户头像（点击进入个人资料）
- 聊天列表
- 通讯录（好友管理）
- 群聊管理
- 文件管理
- **WebSocket 状态指示器**（点击可手动重连/断开）
- 设置

### 会话列表
- 搜索功能
- 添加菜单（添加好友/创建群聊/使用邀请码）
- 会话卡片（头像、名称、最后消息、时间）

### 聊天区域
- 聊天头部（名称、成员数、操作按钮）
- 消息列表（支持图片/视频/文件预览）
- 输入区域（表情、文件附件、发送按钮）

### 信息面板
- 好友/群聊详细信息
- 群成员列表
- 群管理功能

## 🚀 快速开始

### 1. 启动后端服务

```bash
# 启动数据库和 MinIO
podman-compose up -d

# 启动后端
source 当前env
cargo run
```

### 2. 启动 Web 客户端

直接用浏览器打开 `index.html`，或使用简单的 HTTP 服务器：

```bash
# 使用 Python
cd test-web
python -m http.server 8888

# 或使用 Node.js
npx serve -p 8888
```

### 3. 访问应用

打开浏览器访问：`http://localhost:8888`

## 📁 文件结构

```
test-web/
├── index.html      # 主页面（三栏布局 + 模态框）
├── styles.css      # 样式文件（现代化设计）
├── app.js          # 应用逻辑（认证、消息、群聊、文件）
├── modules/        # 模块化 JS（备用）
│   ├── state.js
│   ├── utils.js
│   ├── ui.js
│   ├── auth.js
│   ├── friends.js
│   ├── messages.js
│   └── storage.js
└── README.md       # 本文档
```

## 🔧 配置

### API 基础 URL

默认通过 Nginx 代理访问后端：

```javascript
// app.js
const BASE_URL = 'http://localhost';  // Nginx 代理端口
```

如果需要直接访问后端（不经过 Nginx），修改为：

```javascript
const BASE_URL = 'http://localhost:8080';  // 后端直连端口
```

### 主题适配

支持系统暗色主题自动切换：

```css
@media (prefers-color-scheme: dark) {
  /* 暗色主题样式 */
}
```

## 🎯 功能清单

| 功能模块 | 功能点 | 状态 |
|---------|--------|------|
| **认证** | 注册/登录 | ✅ |
| | Token 刷新 | ✅ |
| | 设备管理 | ✅ |
| | 登出 | ✅ |
| **好友** | 添加好友 | ✅ |
| | 好友请求处理 | ✅ |
| | 好友列表 | ✅ |
| | 删除好友 | ✅ |
| **私聊** | 文本消息 | ✅ |
| | 文件消息 | ✅ |
| | 消息撤回 | ✅ |
| | 消息历史 | ✅ |
| **群聊** | 创建群聊 | ✅ |
| | 群成员管理 | ✅ |
| | 群公告 | ✅ |
| | 邀请码 | ✅ |
| | 群消息 | ✅ |
| | 退出/解散 | ✅ |
| **文件** | 文件上传 | ✅ |
| | 秒传 | ✅ |
| | 文件列表 | ✅ |
| | 文件下载 | ✅ |
| **资料** | 头像上传 | ✅ |
| | 信息修改 | ✅ |
| | 密码修改 | ✅ |
| **WebSocket** | 实时消息推送 | ✅ |
| | 未读消息摘要 | ✅ |
| | 已读同步 | ✅ |
| | 自动重连 | ✅ |
| | 系统通知 | ✅ |
| | 浏览器通知 | ✅ |

## 🛠️ 技术栈

- **HTML5** - 语义化标签
- **CSS3** - Flexbox 布局、CSS 变量、动画
- **JavaScript ES6+** - async/await、模块化
- **Fetch API** - HTTP 请求
- **Web Crypto API** - SHA-256 哈希计算
- **LocalStorage** - 本地状态持久化

## 📝 API 接口

### 认证相关
- `POST /api/auth/register` - 注册
- `POST /api/auth/login` - 登录
- `POST /api/auth/logout` - 登出
- `POST /api/auth/refresh` - 刷新 Token
- `GET /api/auth/devices` - 设备列表
- `DELETE /api/auth/devices/:id` - 删除设备

### 好友相关
- `GET /api/friends` - 好友列表
- `POST /api/friends/requests` - 发送请求
- `GET /api/friends/requests/pending` - 待处理请求
- `POST /api/friends/requests/approve` - 同意请求
- `POST /api/friends/requests/reject` - 拒绝请求
- `POST /api/friends/remove` - 删除好友

### 消息相关
- `POST /api/messages` - 发送消息
- `GET /api/messages` - 获取消息
- `POST /api/messages/recall` - 撤回消息
- `DELETE /api/messages/delete` - 删除消息

### 群聊相关
- `POST /api/groups` - 创建群聊
- `GET /api/groups/my` - 我的群聊
- `GET /api/groups/:id` - 群详情
- `POST /api/groups/:id/invite` - 邀请成员
- `GET /api/groups/:id/members` - 成员列表
- `POST /api/groups/:id/admins` - 设置管理员
- `POST /api/groups/:id/notices` - 发布公告
- `GET /api/groups/:id/notices` - 公告列表
- `POST /api/groups/:id/invite_codes` - 生成邀请码
- `POST /api/groups/join_by_code` - 使用邀请码
- `POST /api/groups/:id/leave` - 退出群聊
- `DELETE /api/groups/:id` - 解散群聊

### 群消息相关
- `POST /api/group-messages` - 发送群消息
- `GET /api/group-messages` - 获取群消息
- `POST /api/group-messages/recall` - 撤回群消息

### 文件相关
- `POST /api/storage/upload/request` - 请求上传
- `GET /api/storage/files` - 文件列表
- `POST /api/storage/file/:uuid/presigned_url` - 获取预签名 URL
- `POST /api/storage/friends_file/:uuid/presigned_url` - 好友文件 URL

### 资料相关
- `GET /api/profile` - 获取资料
- `PUT /api/profile` - 更新资料
- `PUT /api/profile/password` - 修改密码
- `POST /api/profile/avatar` - 上传头像

## 📄 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！
