# Nginx 反向代理配置

## 📋 概述

Nginx 作为 HuanVae Chat 的统一入口，负责：
- 反向代理后端 API 服务
- 反向代理 MinIO 文件存储服务
- SSL/TLS 终端（生产环境）
- 请求负载均衡
- 静态资源缓存

## 🏗️ 架构设计

```
                            外部请求
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Nginx (80/443)                                       │
│                    统一入口 + 反向代理 + SSL终端                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐       ┌───────────────┐       ┌───────────────┐
│  /api/*       │       │  /bucket/*    │       │  /minio/*     │
│  后端 API     │       │  MinIO 文件   │       │  MinIO 控制台 │
│  backend:8080 │       │  minio:9000   │       │  minio:9001   │
└───────────────┘       └───────────────┘       └───────────────┘
                        （user-file, friends-file,
                         group-file, avatars）
```

## 🗂️ 目录结构

```
Nginx/
├── nginx.conf          # 主配置文件
├── conf.d/             # 额外配置（可选）
├── ssl/                # SSL证书目录
│   ├── cert.pem        # 证书文件（生产环境）
│   └── key.pem         # 私钥文件（生产环境）
├── logs/               # 日志目录
│   ├── access.log      # 访问日志
│   └── error.log       # 错误日志
└── README.md           # 本文档
```

## 🔗 路由规则

| 路径 | 目标服务 | 用途 |
|------|---------|------|
| `/api/*` | backend:8080 | 后端 API 服务 |
| `/user-file/*` | minio:9000 | 用户个人文件（预签名URL） |
| `/friends-file/*` | minio:9000 | 好友聊天文件（预签名URL） |
| `/group-file/*` | minio:9000 | 群聊文件（预签名URL） |
| `/avatars/*` | minio:9000 | 公开头像（直接访问） |
| `/minio/*` | minio:9001 | MinIO 管理控制台（开发环境） |
| `/health` | 本地 | Nginx 健康检查 |

## 📝 配置说明

### 上游服务

```nginx
upstream backend {
    server host.containers.internal:8080;  # Rust 后端（宿主机运行）
    keepalive 32;
}

upstream minio_api {
    server minio:9000;  # MinIO API（容器内）
    keepalive 32;
}

upstream minio_console {
    server minio:9001;  # MinIO 控制台（容器内）
    keepalive 32;
}
```

### 后端 API 代理

```nginx
location /api/ {
    proxy_pass http://backend;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

### MinIO 文件代理

```nginx
# 匹配所有 bucket 路径: user-file, friends-file, group-file, avatars
location ~ ^/(user-file|friends-file|group-file|avatars)/ {
    proxy_pass http://minio_api;
    proxy_http_version 1.1;
    # Host 头与 MINIO_PRESIGN_ENDPOINT 保持一致（用于签名验证）
    proxy_set_header Host "localhost";
    proxy_buffering off;           # 大文件支持
    client_max_body_size 0;        # 无大小限制
}
```

### 公开头像（带缓存）

```nginx
location /avatars/ {
    proxy_pass http://minio_api/avatars/;
    proxy_cache_valid 200 1d;
    expires 1d;
    add_header Cache-Control "public, max-age=86400";
}
```

## 🔐 安全配置

### 安全头

```nginx
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
```

### MinIO 控制台访问限制（生产环境建议）

```nginx
location /minio/ {
    # 限制只允许内网访问
    allow 127.0.0.1;
    allow 10.0.0.0/8;
    allow 172.16.0.0/12;
    allow 192.168.0.0/16;
    deny all;
    
    # ... 代理配置
}
```

## 🚀 使用方式

### 开发环境

1. 确保 `compose.yaml` 中 Nginx 服务已配置
2. 启动服务：
   ```bash
   podman-compose up -d
   ```
3. 访问：
   - API: `http://localhost/api/...`
   - 文件: `http://localhost/storage/...`
   - 头像: `http://localhost/avatars/...`
   - MinIO控制台: `http://localhost/minio/`

### 生产环境

1. 将 SSL 证书放入 `ssl/` 目录
2. 取消注释 HTTPS 服务器配置
3. 修改 `server_name` 为实际域名
4. 重启 Nginx

## 📊 访问路径对照表

| 功能 | 修改前 | 修改后（通过Nginx） |
|------|--------|---------------------|
| 后端 API | `http://localhost:8080/api/...` | `http://localhost/api/...` |
| 预签名文件 | `http://localhost:9000/bucket/...` | `http://localhost/storage/bucket/...` |
| 公开头像 | `http://localhost:9000/avatars/...` | `http://localhost/avatars/...` |
| MinIO 控制台 | `http://localhost:9001` | `http://localhost/minio/` |

## ⚙️ 环境变量配置

更新 `.env` 文件：

```bash
# MinIO 配置
MINIO_ENDPOINT=http://minio:9000           # 内部通信（容器名）
MINIO_PUBLIC_URL=http://localhost/storage  # 外部访问（通过Nginx）

# 生产环境
# MINIO_PUBLIC_URL=https://your-domain.com/storage
```

## 🔍 故障排查

### 检查 Nginx 状态

```bash
# 查看容器状态
podman ps | grep nginx

# 查看日志
podman logs huanvae-nginx

# 测试配置
podman exec huanvae-nginx nginx -t
```

### 常见问题

1. **502 Bad Gateway**
   - 检查后端服务是否运行
   - 检查 `host.containers.internal` 解析是否正常

2. **文件上传失败**
   - 检查 `client_max_body_size` 配置
   - 检查 MinIO 服务是否正常

3. **预签名URL无法访问**
   - 确认 `MINIO_PUBLIC_URL` 配置正确
   - 检查 `/storage/` 路由是否正常

## 📖 相关文档

- [compose.yaml](../compose.yaml) - Docker Compose 配置
- [MinIO README](../MinIO/README.md) - MinIO 存储说明
- [Storage README](../src/storage/README.md) - 存储服务说明

