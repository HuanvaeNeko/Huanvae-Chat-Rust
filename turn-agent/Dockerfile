# Turn Agent Dockerfile
# 多阶段构建，生成轻量级镜像

# ============================================
# 阶段 1: 构建
# ============================================
FROM rust:1.91-alpine AS builder

# 安装构建依赖
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /build

# 先复制依赖文件，利用 Docker 缓存
COPY Cargo.toml ./

# 创建空的 src/main.rs 用于预编译依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm -rf src

# 复制实际源码
COPY src ./src

# 编译
RUN cargo build --release

# ============================================
# 阶段 2: 运行
# ============================================
FROM alpine:3.20

# 安装运行时依赖
RUN apk add --no-cache ca-certificates curl procps

WORKDIR /app

# 复制二进制文件
COPY --from=builder /build/target/release/turn-agent /app/turn-agent

# 复制配置模板
COPY config/turnserver.conf.template /app/templates/

# 复制健康检查脚本
COPY scripts/health-check.sh /app/health-check
RUN chmod +x /app/health-check

# 创建数据目录
RUN mkdir -p /etc/turnserver /var/log/turnserver

# 设置环境变量默认值
ENV LOG_LEVEL=info
ENV HEARTBEAT_INTERVAL=5

ENTRYPOINT ["/app/turn-agent"]

