#!/bin/bash

# Rust 镜像源一键配置脚本（清华大学镜像站）
# 作者: Huanwei
# 日期: 2025-11-28
# 参考: https://mirrors.tuna.tsinghua.edu.cn/help/rustup/
#       https://mirrors.tuna.tsinghua.edu.cn/help/crates.io-index.git/

set -e

echo "========================================"
echo "  Rust 镜像源一键配置工具"
echo "  镜像站: 清华大学开源软件镜像站"
echo "========================================"
echo ""

# 定义镜像地址
RUSTUP_MIRROR="https://mirrors.tuna.tsinghua.edu.cn/rustup"
CRATES_MIRROR="sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

# 确定 CARGO_HOME 路径
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

echo "📁 Cargo 配置目录: $CARGO_HOME"
echo ""

# 检查是否安装了 Rust
if command -v rustc &> /dev/null; then
    echo "✅ 检测到 Rust 版本: $(rustc --version)"
else
    echo "⚠️  警告: 未检测到 Rust，请先安装 Rust"
    echo "   安装命令: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
echo ""

# 创建 cargo 配置目录
mkdir -p "$CARGO_HOME"

# 备份原有配置
CONFIG_FILE="$CARGO_HOME/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    BACKUP_FILE="${CONFIG_FILE}.backup.$(date +%Y%m%d_%H%M%S)"
    echo "📦 备份原配置文件到: $BACKUP_FILE"
    cp "$CONFIG_FILE" "$BACKUP_FILE"
fi

# 生成配置文件
echo "🔧 生成 Cargo 镜像配置..."
cat > "$CONFIG_FILE" <<EOF
# Rust Cargo 镜像配置
# 自动生成时间: $(date '+%Y-%m-%d %H:%M:%S')
# 镜像站: 清华大学开源软件镜像站
# 参考: https://mirrors.tuna.tsinghua.edu.cn/

# 使用清华镜像源替代 crates.io
[source.crates-io]
replace-with = 'tuna'

# 清华大学 crates.io 镜像（稀疏索引）
[source.tuna]
registry = "$CRATES_MIRROR"

# 注册表协议（推荐使用 sparse 协议以提高性能）
[registries.crates-io]
protocol = "sparse"

# 网络配置
[net]
git-fetch-with-cli = false

# HTTP 配置
[http]
check-revoke = false
multiplexing = true

# 构建配置
[build]
# 并行构建数量（根据 CPU 核心数自动调整）
jobs = $(nproc 2>/dev/null || echo 4)
EOF

echo "✅ Cargo 配置完成！"
echo ""

# 配置 shell 环境变量（用于 rustup）
echo "🔧 配置 Rustup 环境变量..."

# 检测当前 shell
CURRENT_SHELL=$(basename "$SHELL")

case $CURRENT_SHELL in
    bash)
        PROFILE_FILE="$HOME/.bashrc"
        echo "" >> "$PROFILE_FILE"
        echo "# Rust Rustup 镜像配置 ($(date '+%Y-%m-%d'))" >> "$PROFILE_FILE"
        echo "export RUSTUP_UPDATE_ROOT=$RUSTUP_MIRROR/rustup" >> "$PROFILE_FILE"
        echo "export RUSTUP_DIST_SERVER=$RUSTUP_MIRROR" >> "$PROFILE_FILE"
        echo "✅ 已添加环境变量到: $PROFILE_FILE"
        ;;
    zsh)
        PROFILE_FILE="$HOME/.zshrc"
        echo "" >> "$PROFILE_FILE"
        echo "# Rust Rustup 镜像配置 ($(date '+%Y-%m-%d'))" >> "$PROFILE_FILE"
        echo "export RUSTUP_UPDATE_ROOT=$RUSTUP_MIRROR/rustup" >> "$PROFILE_FILE"
        echo "export RUSTUP_DIST_SERVER=$RUSTUP_MIRROR" >> "$PROFILE_FILE"
        echo "✅ 已添加环境变量到: $PROFILE_FILE"
        ;;
    fish)
        PROFILE_FILE="$HOME/.config/fish/config.fish"
        mkdir -p "$(dirname "$PROFILE_FILE")"
        echo "" >> "$PROFILE_FILE"
        echo "# Rust Rustup 镜像配置 ($(date '+%Y-%m-%d'))" >> "$PROFILE_FILE"
        echo "set -x RUSTUP_UPDATE_ROOT $RUSTUP_MIRROR/rustup" >> "$PROFILE_FILE"
        echo "set -x RUSTUP_DIST_SERVER $RUSTUP_MIRROR" >> "$PROFILE_FILE"
        echo "✅ 已添加环境变量到: $PROFILE_FILE"
        ;;
    *)
        echo "⚠️  检测到 Shell: $CURRENT_SHELL"
        echo "   请手动添加以下环境变量到您的 shell 配置文件："
        echo ""
        echo "   export RUSTUP_UPDATE_ROOT=$RUSTUP_MIRROR/rustup"
        echo "   export RUSTUP_DIST_SERVER=$RUSTUP_MIRROR"
        ;;
esac

echo ""
echo "========================================"
echo "  ✅ 配置完成！"
echo "========================================"
echo ""
echo "📋 已配置的镜像源："
echo "   • Cargo 源: $CRATES_MIRROR"
echo "   • Rustup 源: $RUSTUP_MIRROR"
echo ""
echo "💡 使用说明："
echo ""
echo "1️⃣  立即生效（当前 Shell）："
echo "   source ~/${PROFILE_FILE#$HOME/}"
echo ""
echo "2️⃣  测试 Cargo 镜像："
echo "   cargo search tokio"
echo ""
echo "3️⃣  测试 Rustup 镜像："
echo "   rustup update"
echo ""
echo "4️⃣  清理缓存（如遇问题）："
echo "   rm -rf ~/.cargo/.package-cache"
echo "   rm -rf ~/.cargo/registry"
echo ""
echo "5️⃣  查看当前配置："
echo "   cat $CONFIG_FILE"
echo ""
echo "📌 注意事项："
echo "   - 配置在新终端会话中自动生效"
echo "   - 原配置文件已备份（如果存在）"
echo "   - 镜像源仅保留一段时间的 nightly 版本"
echo ""
echo "🔗 参考文档："
echo "   https://mirrors.tuna.tsinghua.edu.cn/help/rustup/"
echo "   https://mirrors.tuna.tsinghua.edu.cn/help/crates.io-index.git/"
echo ""
echo "========================================"
echo "  祝您开发愉快！"
echo "========================================"

