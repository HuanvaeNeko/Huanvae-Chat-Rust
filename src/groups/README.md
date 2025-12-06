# Groups 群聊模块

群聊管理模块，提供完整的群聊功能，包括创建群聊、成员管理、角色管理、入群机制等。

## 📂 目录结构

```
src/groups/
├── handlers/             # HTTP 请求处理器层
│   ├── routes.rs         # 路由定义
│   ├── state.rs          # GroupsState（Handler 层状态）
│   ├── create_group.rs   # 创建群聊
│   ├── get_group.rs      # 获取群信息、群列表、搜索
│   ├── update_group.rs   # 更新群信息、入群模式
│   ├── disband_group.rs  # 解散群聊
│   ├── members.rs        # 成员管理（邀请、退出、移除）
│   ├── roles.rs          # 角色管理（转让群主、设置管理员）
│   ├── mute.rs           # 禁言管理
│   ├── invite_codes.rs   # 邀请码管理
│   ├── join_requests.rs  # 入群申请处理
│   └── notices.rs        # 群公告管理
├── models/               # 数据模型
│   ├── group.rs          # 群聊模型
│   ├── member.rs         # 成员模型
│   ├── request.rs        # 请求模型
│   ├── response.rs       # 响应模型
│   ├── invite_code.rs    # 邀请码模型
│   └── notice.rs         # 公告模型
├── services/             # 业务逻辑
│   ├── group_service.rs  # 群聊核心服务
│   ├── member_service.rs # 成员管理服务
│   ├── invite_code_service.rs # 邀请码服务
│   └── notice_service.rs # 公告服务
├── mod.rs
└── README.md
```

## 🔗 路由映射

### 群聊基础操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups` | 创建群聊 |
| GET | `/api/groups/my` | 获取我加入的群聊列表 |
| GET | `/api/groups/search?keyword=xxx` | 搜索群聊 |
| GET | `/api/groups/:group_id` | 获取群聊信息 |
| PUT | `/api/groups/:group_id` | 更新群聊信息 |
| DELETE | `/api/groups/:group_id` | 解散群聊（仅群主） |
| PUT | `/api/groups/:group_id/join_mode` | 修改入群模式（仅群主） |
| POST | `/api/groups/:group_id/avatar` | 上传群头像（群主/管理员） |
| PUT | `/api/groups/:group_id/nickname` | 修改群内昵称（群成员） |

### 成员管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/groups/:group_id/members` | 获取成员列表 |
| POST | `/api/groups/:group_id/invite` | 邀请成员入群 |
| POST | `/api/groups/:group_id/leave` | 退出群聊 |
| DELETE | `/api/groups/:group_id/members/:user_id` | 移除成员 |

### 角色管理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups/:group_id/transfer` | 转让群主 |
| POST | `/api/groups/:group_id/admins` | 设置管理员 |
| DELETE | `/api/groups/:group_id/admins/:user_id` | 取消管理员 |

### 禁言管理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups/:group_id/mute` | 禁言成员 |
| DELETE | `/api/groups/:group_id/mute/:user_id` | 解除禁言 |

### 邀请码管理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups/:group_id/invite-codes` | 生成邀请码 |
| GET | `/api/groups/:group_id/invite-codes` | 获取邀请码列表 |
| DELETE | `/api/groups/:group_id/invite-codes/:code_id` | 撤销邀请码 |
| POST | `/api/groups/join-by-code` | 通过邀请码入群 |

### 入群申请

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups/:group_id/apply` | 申请入群（搜索方式） |
| GET | `/api/groups/:group_id/requests` | 获取待处理申请（管理员） |
| POST | `/api/groups/:group_id/requests/:id/approve` | 同意申请 |
| POST | `/api/groups/:group_id/requests/:id/reject` | 拒绝申请 |
| GET | `/api/groups/invitations` | 获取收到的邀请 |
| POST | `/api/groups/invitations/:id/accept` | 接受邀请 |
| POST | `/api/groups/invitations/:id/decline` | 拒绝邀请 |

### 群公告

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/groups/:group_id/notices` | 发布公告 |
| GET | `/api/groups/:group_id/notices` | 获取公告列表 |
| PUT | `/api/groups/:group_id/notices/:notice_id` | 更新公告 |
| DELETE | `/api/groups/:group_id/notices/:notice_id` | 删除公告 |

## 🗄️ 数据库设计

### groups 表（群聊主表）

| 字段 | 类型 | 说明 |
|------|------|------|
| group-id | UUID | 群聊唯一标识 |
| group-name | TEXT | 群名称 |
| group-avatar-url | TEXT | 群头像URL |
| group-description | TEXT | 群简介 |
| creator-id | TEXT | 创建人ID |
| created-at | TIMESTAMPTZ | 创建时间 |
| join-mode | TEXT | 入群模式 |
| status | TEXT | 状态：active/disbanded |
| member-count | INTEGER | 成员数量 |

### group-members 表（群成员表）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 记录ID |
| group-id | UUID | 群聊ID |
| user-id | TEXT | 用户ID |
| role | TEXT | 角色：owner/admin/member |
| group-nickname | TEXT | 群内昵称（可选） |
| join-method | TEXT | 入群方式 |
| status | TEXT | 状态：active/removed/left |
| muted-until | TIMESTAMPTZ | 禁言截止时间 |

### 入群模式（join-mode）

| 值 | 说明 |
|---|---|
| open | 开放入群：所有方式均可直接进入 |
| approval_required | 需审核：普通成员邀请和搜索入群需审核 |
| invite_only | 仅邀请：只能通过邀请进入 |
| admin_invite_only | 仅管理邀请：只能群主/管理员邀请进入 |
| closed | 关闭入群：不允许任何新成员加入 |

### 入群方式（join-method）

| 值 | 说明 |
|---|---|
| create | 创建群时自动加入（群主） |
| owner_invite | 群主邀请 |
| admin_invite | 管理员邀请 |
| member_invite | 普通成员邀请（经审核） |
| direct_invite_code | 通过直通邀请码 |
| normal_invite_code | 通过普通邀请码（经审核） |
| search_direct | 通过搜索直接加入 |
| search_approved | 通过搜索申请（经审核） |

## 🔐 权限控制

| 操作 | 群主 | 管理员 | 普通成员 |
|------|-----|-------|---------|
| 修改群信息 | ✅ | ✅ | ❌ |
| 修改入群模式 | ✅ | ❌ | ❌ |
| 上传群头像 | ✅ | ✅ | ❌ |
| 修改群内昵称 | ✅ | ✅ | ✅ |
| 邀请成员（直接） | ✅ | ✅ | ❌ |
| 邀请成员（需审核） | - | - | ✅ |
| 生成直通邀请码 | ✅ | ✅ | ❌ |
| 生成普通邀请码 | ✅ | ✅ | ✅ |
| 审核入群申请 | ✅ | ✅ | ❌ |
| 设置管理员 | ✅ | ❌ | ❌ |
| 移除成员 | ✅ | ✅（不能移除管理员） | ❌ |
| 禁言成员 | ✅ | ✅（不能禁言管理员） | ❌ |
| 发布公告 | ✅ | ✅ | ❌ |
| 转让群主 | ✅ | ❌ | ❌ |
| 解散群聊 | ✅ | ❌ | ❌ |

### 权限验证 API

`MemberService` 提供统一的权限验证方法 `check_permission()`：

```rust
use crate::groups::models::RequiredPermission;

// 统一的权限验证（推荐使用）
let has_permission = member_service
    .check_permission(&group_id, &user_id, RequiredPermission::AdminOrOwner)
    .await?;

// 便捷方法（内部调用 check_permission）
let is_active = member_service.verify_active_member(&group_id, &user_id).await?;
let is_admin = member_service.verify_admin_or_owner(&group_id, &user_id).await?;
let is_owner = member_service.verify_owner(&group_id, &user_id).await?;
```

**RequiredPermission 枚举**：

| 值 | 说明 |
|---|---|
| `ActiveMember` | 任何活跃群成员 |
| `AdminOrOwner` | 管理员或群主 |
| `OwnerOnly` | 仅群主 |

## 🔄 业务流程

### 转让群主

群主可以将群主头衔移交给任何群成员：
- 如果新群主是管理员 → 从管理员变为群主
- 如果新群主是普通成员 → 直接变为群主
- 旧群主变为普通成员

### 退群流程

- 普通成员/管理员可以主动退出
- 群主不能退出，需先转让群主或解散群聊
- 退群记录保留（软删除）

### 解散群聊

- 仅群主可以解散
- 软删除，保留所有记录
- 所有成员状态标记为 left
- 撤销所有邀请码
- 取消所有待处理申请

## 📝 API 示例

### 创建群聊

```bash
curl -X POST http://localhost:8080/api/groups \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "group_name": "测试群聊",
    "group_description": "这是一个测试群",
    "join_mode": "approval_required"
  }'
```

### 邀请成员

```bash
curl -X POST http://localhost:8080/api/groups/{group_id}/invite \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "user_ids": ["user_a", "user_b"],
    "message": "邀请你加入群聊"
  }'
```

### 转让群主

```bash
curl -X POST http://localhost:8080/api/groups/{group_id}/transfer \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "new_owner_id": "user_b"
  }'
```

### 上传群头像

```bash
curl -X POST http://localhost:8080/api/groups/{group_id}/avatar \
  -H "Authorization: Bearer <token>" \
  -F "avatar=@/path/to/image.jpg"
```

### 修改群内昵称

```bash
curl -X PUT http://localhost:8080/api/groups/{group_id}/nickname \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "nickname": "我在这个群的昵称"
  }'
```

> **说明**: `nickname` 设为 `null` 或空字符串表示清除群内昵称，恢复显示用户全局昵称。

