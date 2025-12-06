#!/bin/bash
# Turn Agent 一键部署脚本

set -e

echo "========================================"
echo "  Turn Agent 一键部署脚本"
echo "========================================"
echo ""

# 切换到脚本所在目录的父目录
cd "$(dirname "$0")/.."

# 检查是否存在 .env
if [ ! -f .env ]; then
    echo "📝 首次部署，正在创建配置文件..."
    cp .env.example .env
    
    echo ""
    echo "⚠️  请编辑 .env 文件，配置以下必填项："
    echo ""
    echo "   PUBLIC_IP         - 本机公网 IP"
    echo "   COORDINATOR_URL   - 主服务器 WebSocket 地址"
    echo "   COORDINATOR_TOKEN - Agent 认证令牌"
    echo ""
    echo "配置完成后，重新运行此脚本: ./scripts/setup.sh"
    exit 0
fi

# 加载配置
source .env

# 验证必填配置
echo "🔍 验证配置..."

if [ -z "$PUBLIC_IP" ] || [ "$PUBLIC_IP" = "你的公网IP" ]; then
    echo "❌ 错误: 请在 .env 中配置 PUBLIC_IP"
    exit 1
fi

if [ -z "$COORDINATOR_URL" ] || [[ "$COORDINATOR_URL" == *"example.com"* ]]; then
    echo "❌ 错误: 请在 .env 中配置 COORDINATOR_URL"
    exit 1
fi

if [ -z "$COORDINATOR_TOKEN" ] || [ "$COORDINATOR_TOKEN" = "your-agent-token-here" ]; then
    echo "❌ 错误: 请在 .env 中配置 COORDINATOR_TOKEN"
    exit 1
fi

echo ""
echo "✅ 配置验证通过:"
echo "   节点 ID:   ${NODE_ID:-auto}"
echo "   区域:      ${REGION:-unknown}"
echo "   公网 IP:   $PUBLIC_IP"
echo "   服务器:    $COORDINATOR_URL"
echo ""

# 创建数据目录
echo "📁 创建数据目录..."
mkdir -p data/config data/logs

# 检查 Docker/Podman
if command -v podman-compose &> /dev/null; then
    COMPOSE_CMD="podman-compose"
elif command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
elif command -v docker &> /dev/null && docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
else
    echo "❌ 错误: 未找到 docker-compose 或 podman-compose"
    exit 1
fi

echo "🐳 使用: $COMPOSE_CMD"
echo ""

# 构建并启动服务
echo "🚀 正在启动服务..."
$COMPOSE_CMD up -d --build

echo ""
echo "========================================"
echo "  ✅ 部署完成！"
echo "========================================"
echo ""
echo "📋 常用命令:"
echo "   查看日志:     $COMPOSE_CMD logs -f"
echo "   查看状态:     $COMPOSE_CMD ps"
echo "   停止服务:     $COMPOSE_CMD down"
echo "   重启服务:     $COMPOSE_CMD restart"
echo ""
echo "📊 Agent 日志:   $COMPOSE_CMD logs -f turn-agent"
echo "📊 Coturn 日志:  $COMPOSE_CMD logs -f coturn"
echo ""

