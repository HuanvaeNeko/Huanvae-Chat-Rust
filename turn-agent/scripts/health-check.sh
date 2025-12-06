#!/bin/sh
# Turn Agent 健康检查脚本

# 检查配置文件是否存在
if [ ! -f /etc/turnserver/turnserver.conf ]; then
    echo "Config file not found"
    exit 1
fi

# 检查配置文件是否有内容
if [ ! -s /etc/turnserver/turnserver.conf ]; then
    echo "Config file is empty"
    exit 1
fi

# 检查 turn-agent 进程是否运行
if ! pgrep -x "turn-agent" > /dev/null 2>&1; then
    echo "turn-agent process not running"
    exit 1
fi

echo "OK"
exit 0

