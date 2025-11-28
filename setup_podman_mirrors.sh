#!/bin/bash

# Podman Docker Hub 镜像加速一键配置脚本
# 作者: Huanwei
# 日期: 2025-11-28

set -e

echo "========================================"
echo "  Podman Docker Hub 镜像加速配置工具"
echo "========================================"
echo ""

# 定义镜像源列表
MIRRORS=(
    "https://docker.1ms.run"
    "https://docker.xpg666.xyz"
    "https://lispy.org"
    "https://docker.xiaogenban1993.com"
    "https://docker-0.unsee.tech"
    "https://666860.xyz"
)

# 检查是否安装了 Podman
if ! command -v podman &> /dev/null; then
    echo "❌ 错误: 未检测到 Podman，请先安装 Podman"
    exit 1
fi

echo "✅ 检测到 Podman 版本: $(podman --version)"
echo ""

# 确定配置文件路径
if [ "$EUID" -eq 0 ]; then
    # 以 root 运行，使用系统级配置
    CONFIG_DIR="/etc/containers"
    CONFIG_FILE="$CONFIG_DIR/registries.conf"
    echo "📁 使用系统级配置路径: $CONFIG_FILE"
else
    # 普通用户，使用用户级配置
    CONFIG_DIR="$HOME/.config/containers"
    CONFIG_FILE="$CONFIG_DIR/registries.conf"
    echo "📁 使用用户级配置路径: $CONFIG_FILE"
fi

# 备份原配置文件
if [ -f "$CONFIG_FILE" ]; then
    BACKUP_FILE="${CONFIG_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
    echo "📦 备份原配置文件到: $BACKUP_FILE"
    cp "$CONFIG_FILE" "$BACKUP_FILE"
fi

# 创建配置目录
mkdir -p "$CONFIG_DIR"

# 生成镜像配置
echo "🔧 生成镜像加速配置..."
cat > "$CONFIG_FILE" <<EOF
# Podman 镜像加速配置
# 自动生成时间: $(date '+%Y-%m-%d %H:%M:%S')
# 
# 本配置文件配置了多个 Docker Hub 镜像源以加速镜像拉取

# 取消限定的注册表列表（可选）
unqualified-search-registries = ["docker.io"]

# Docker Hub 镜像配置
[[registry]]
prefix = "docker.io"
location = "docker.io"

EOF

# 添加所有镜像源
for mirror in "${MIRRORS[@]}"; do
    # 移除 https:// 前缀（如果存在）
    mirror_location="${mirror#https://}"
    mirror_location="${mirror_location#http://}"
    
    cat >> "$CONFIG_FILE" <<EOF
# 镜像源: $mirror
[[registry.mirror]]
location = "$mirror_location"

EOF
done

echo ""
echo "✅ 配置完成！已添加 ${#MIRRORS[@]} 个镜像源："
echo ""
for mirror in "${MIRRORS[@]}"; do
    echo "   • $mirror"
done

echo ""
echo "📋 配置文件位置: $CONFIG_FILE"
echo ""
echo "🧪 测试镜像拉取："
echo "   podman pull hello-world"
echo ""
echo "💡 提示："
echo "   - 配置已自动生效，无需重启服务"
echo "   - 原配置文件已备份（如果存在）"
echo "   - 如需恢复原配置，可使用备份文件"
echo ""
echo "🔍 查看当前配置："
echo "   cat $CONFIG_FILE"
echo ""
echo "========================================"
echo "  配置完成！祝使用愉快！"
echo "========================================"

