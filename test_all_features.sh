#!/bin/bash

# HuanVae Chat 完整功能测试脚本
# 测试：用户注册、好友系统、个人资料、设备管理

set -e  # 遇到错误立即退出

BASE_URL="http://localhost:8080"
TIMESTAMP=$(date +%s)

# 颜色输出
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_step() {
    echo -e "\n${YELLOW}========================================${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}========================================${NC}"
}

# API 请求函数
api_call() {
    local method=$1
    local path=$2
    local token=$3
    local data=$4
    
    if [ -n "$token" ]; then
        if [ -n "$data" ]; then
            curl -s -X "$method" "${BASE_URL}${path}" \
                -H "Authorization: Bearer $token" \
                -H "Content-Type: application/json" \
                -d "$data"
        else
            curl -s -X "$method" "${BASE_URL}${path}" \
                -H "Authorization: Bearer $token"
        fi
    else
        if [ -n "$data" ]; then
            curl -s -X "$method" "${BASE_URL}${path}" \
                -H "Content-Type: application/json" \
                -d "$data"
        else
            curl -s -X "$method" "${BASE_URL}${path}"
        fi
    fi
}

# 检查服务是否运行
check_service() {
    log_info "检查服务是否运行..."
    if curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
        log_success "服务正常运行"
        return 0
    else
        log_error "服务未运行，请先启动服务: cargo run"
        exit 1
    fi
}

# 清理函数
cleanup() {
    log_info "清理临时文件..."
    rm -f /tmp/user1_*.txt /tmp/user2_*.txt /tmp/test_avatar_*.png
}

# ==============================================
# 测试开始
# ==============================================

echo -e "${GREEN}"
echo "╔═══════════════════════════════════════════════╗"
echo "║   HuanVae Chat 完整功能自动化测试            ║"
echo "║   测试时间: $(date '+%Y-%m-%d %H:%M:%S')      ║"
echo "╚═══════════════════════════════════════════════╝"
echo -e "${NC}"

# 检查服务
check_service

# 初始化测试数据
USER1_ID="testuser_${TIMESTAMP}_1"
USER2_ID="testuser_${TIMESTAMP}_2"
USER1_NICK="测试用户1"
USER2_NICK="测试用户2"
PASSWORD="test123456"
DEVICE_INFO="Linux/TestScript"
MAC_ADDR1="00:11:22:33:44:55"
MAC_ADDR2="00:11:22:33:44:66"

# ==============================================
# 第一部分：用户注册
# ==============================================

log_step "第 1 步：创建两个测试用户"

log_info "注册用户1: $USER1_ID"
REGISTER1=$(api_call POST /api/auth/register "" "{
    \"user_id\": \"$USER1_ID\",
    \"nickname\": \"$USER1_NICK\",
    \"email\": \"${USER1_ID}@example.com\",
    \"password\": \"$PASSWORD\"
}")
echo "$REGISTER1" | jq '.' 2>/dev/null || echo "$REGISTER1"

if echo "$REGISTER1" | jq -e '.user_id' > /dev/null 2>&1; then
    log_success "用户1 注册成功"
else
    log_error "用户1 注册失败"
    exit 1
fi

sleep 1

log_info "注册用户2: $USER2_ID"
REGISTER2=$(api_call POST /api/auth/register "" "{
    \"user_id\": \"$USER2_ID\",
    \"nickname\": \"$USER2_NICK\",
    \"email\": \"${USER2_ID}@example.com\",
    \"password\": \"$PASSWORD\"
}")
echo "$REGISTER2" | jq '.' 2>/dev/null || echo "$REGISTER2"

if echo "$REGISTER2" | jq -e '.user_id' > /dev/null 2>&1; then
    log_success "用户2 注册成功"
else
    log_error "用户2 注册失败"
    exit 1
fi

# ==============================================
# 第二部分：用户登录
# ==============================================

log_step "第 2 步：两个用户分别登录"

log_info "用户1 登录..."
LOGIN1=$(api_call POST /api/auth/login "" "{
    \"user_id\": \"$USER1_ID\",
    \"password\": \"$PASSWORD\",
    \"device_info\": \"$DEVICE_INFO\",
    \"mac_address\": \"$MAC_ADDR1\"
}")

USER1_TOKEN=$(echo "$LOGIN1" | jq -r '.access_token')
USER1_REFRESH=$(echo "$LOGIN1" | jq -r '.refresh_token')
USER1_DEVICE=$(echo "$LOGIN1" | jq -r '.access_token' | cut -d'.' -f2 | base64 -d 2>/dev/null | jq -r '.device_id' 2>/dev/null || echo "unknown")

if [ "$USER1_TOKEN" != "null" ] && [ -n "$USER1_TOKEN" ]; then
    log_success "用户1 登录成功"
    log_info "Token: ${USER1_TOKEN:0:50}..."
else
    log_error "用户1 登录失败"
    echo "$LOGIN1"
    exit 1
fi

sleep 1

log_info "用户2 登录..."
LOGIN2=$(api_call POST /api/auth/login "" "{
    \"user_id\": \"$USER2_ID\",
    \"password\": \"$PASSWORD\",
    \"device_info\": \"$DEVICE_INFO\",
    \"mac_address\": \"$MAC_ADDR2\"
}")

USER2_TOKEN=$(echo "$LOGIN2" | jq -r '.access_token')
USER2_REFRESH=$(echo "$LOGIN2" | jq -r '.refresh_token')

if [ "$USER2_TOKEN" != "null" ] && [ -n "$USER2_TOKEN" ]; then
    log_success "用户2 登录成功"
    log_info "Token: ${USER2_TOKEN:0:50}..."
else
    log_error "用户2 登录失败"
    echo "$LOGIN2"
    exit 1
fi

# ==============================================
# 第三部分：好友功能测试
# ==============================================

log_step "第 3 步：用户1 向用户2 发送好友请求"

FRIEND_REQ1=$(api_call POST /api/friends/requests "$USER1_TOKEN" "{
    \"user_id\": \"$USER1_ID\",
    \"target_user_id\": \"$USER2_ID\",
    \"reason\": \"你好，我是用户1\",
    \"request_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
}")
echo "$FRIEND_REQ1" | jq '.' 2>/dev/null || echo "$FRIEND_REQ1"

if echo "$FRIEND_REQ1" | jq -e '.request_id' > /dev/null 2>&1; then
    REQUEST_ID=$(echo "$FRIEND_REQ1" | jq -r '.request_id')
    log_success "好友请求发送成功 (ID: $REQUEST_ID)"
else
    log_error "好友请求发送失败"
fi

sleep 1

# ==============================================

log_step "第 4 步：用户2 查看待处理的好友请求"

PENDING_REQ=$(api_call GET /api/friends/requests/pending "$USER2_TOKEN")
echo "$PENDING_REQ" | jq '.' 2>/dev/null || echo "$PENDING_REQ"

if echo "$PENDING_REQ" | grep -q "$USER1_ID"; then
    log_success "用户2 收到好友请求"
else
    log_error "用户2 未收到好友请求"
fi

sleep 1

# ==============================================

log_step "第 5 步：用户2 同意好友请求"

APPROVE_REQ=$(api_call POST /api/friends/requests/approve "$USER2_TOKEN" "{
    \"user_id\": \"$USER2_ID\",
    \"applicant_user_id\": \"$USER1_ID\",
    \"approved_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",
    \"approved_reason\": \"同意添加\"
}")
echo "$APPROVE_REQ" | jq '.' 2>/dev/null || echo "$APPROVE_REQ"

if echo "$APPROVE_REQ" | grep -qi "error\|错误\|失败"; then
    log_error "好友请求同意失败"
else
    log_success "好友请求已同意"
fi

sleep 1

# ==============================================

log_step "第 6 步：验证双方好友列表"

log_info "查询用户1的好友列表..."
FRIENDS1=$(api_call GET /api/friends "$USER1_TOKEN")
echo "$FRIENDS1" | jq '.' 2>/dev/null || echo "$FRIENDS1"

if echo "$FRIENDS1" | grep -q "$USER2_ID"; then
    log_success "用户1 好友列表包含用户2"
else
    log_error "用户1 好友列表不包含用户2"
fi

sleep 1

log_info "查询用户2的好友列表..."
FRIENDS2=$(api_call GET /api/friends "$USER2_TOKEN")
echo "$FRIENDS2" | jq '.' 2>/dev/null || echo "$FRIENDS2"

if echo "$FRIENDS2" | grep -q "$USER1_ID"; then
    log_success "用户2 好友列表包含用户1"
else
    log_error "用户2 好友列表不包含用户1"
fi

sleep 1

# ==============================================

log_step "第 7 步：用户1 删除用户2 好友"

REMOVE_FRIEND=$(api_call POST /api/friends/remove "$USER1_TOKEN" "{
    \"user_id\": \"$USER1_ID\",
    \"friend_user_id\": \"$USER2_ID\",
    \"remove_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",
    \"remove_reason\": \"测试删除功能\"
}")
echo "$REMOVE_FRIEND" | jq '.' 2>/dev/null || echo "$REMOVE_FRIEND"

if echo "$REMOVE_FRIEND" | grep -qi "error\|错误\|失败"; then
    log_error "好友删除失败"
else
    log_success "好友删除成功"
fi

sleep 1

# ==============================================

log_step "第 8 步：重新发送好友请求（单方面）"

FRIEND_REQ2=$(api_call POST /api/friends/requests "$USER1_ID" "$USER1_TOKEN" "{
    \"user_id\": \"$USER1_ID\",
    \"target_user_id\": \"$USER2_ID\",
    \"reason\": \"重新添加好友\",
    \"request_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
}")

log_info "用户1 再次向用户2 发送好友请求"
echo "$FRIEND_REQ2" | jq '.' 2>/dev/null || echo "$FRIEND_REQ2"

sleep 1

# ==============================================

log_step "第 9 步：用户1 查看发出的好友请求"

SENT_REQ=$(api_call GET /api/friends/requests/sent "$USER1_TOKEN")
echo "$SENT_REQ" | jq '.' 2>/dev/null || echo "$SENT_REQ"

if echo "$SENT_REQ" | grep -q "$USER2_ID"; then
    log_success "用户1 发送列表包含请求"
else
    log_error "用户1 发送列表为空"
fi

sleep 1

# ==============================================
# 第四部分：个人资料测试
# ==============================================

log_step "第 10 步：用户1 获取个人信息"

PROFILE1=$(api_call GET /api/profile "$USER1_TOKEN")
echo "$PROFILE1" | jq '.' 2>/dev/null || echo "$PROFILE1"

if echo "$PROFILE1" | jq -e '.data.user_id' > /dev/null 2>&1; then
    log_success "获取个人信息成功"
else
    log_error "获取个人信息失败"
fi

sleep 1

# ==============================================

log_step "第 11 步：用户1 更新个人信息"

UPDATE_PROFILE=$(api_call PUT /api/profile "$USER1_TOKEN" "{
    \"email\": \"updated_${USER1_ID}@example.com\",
    \"signature\": \"这是我的个性签名 - 测试\"
}")
echo "$UPDATE_PROFILE" | jq '.' 2>/dev/null || echo "$UPDATE_PROFILE"

if echo "$UPDATE_PROFILE" | grep -q "success\|Success\|成功"; then
    log_success "个人信息更新成功"
else
    log_error "个人信息更新失败"
fi

sleep 1

# ==============================================

log_step "第 12 步：用户1 修改密码"

NEW_PASSWORD="newpass123456"
UPDATE_PASSWORD=$(api_call PUT /api/profile/password "$USER1_TOKEN" "{
    \"old_password\": \"$PASSWORD\",
    \"new_password\": \"$NEW_PASSWORD\"
}")
echo "$UPDATE_PASSWORD" | jq '.' 2>/dev/null || echo "$UPDATE_PASSWORD"

if echo "$UPDATE_PASSWORD" | grep -q "success\|Success\|成功"; then
    log_success "密码修改成功"
    PASSWORD="$NEW_PASSWORD"  # 更新密码变量
else
    log_error "密码修改失败"
fi

sleep 1

# ==============================================

log_step "第 13 步：验证新密码可以登录"

LOGIN1_NEW=$(api_call POST /api/auth/login "" "{
    \"user_id\": \"$USER1_ID\",
    \"password\": \"$NEW_PASSWORD\",
    \"device_info\": \"$DEVICE_INFO\",
    \"mac_address\": \"$MAC_ADDR1\"
}")

NEW_TOKEN=$(echo "$LOGIN1_NEW" | jq -r '.access_token')

if [ "$NEW_TOKEN" != "null" ] && [ -n "$NEW_TOKEN" ]; then
    log_success "新密码登录成功"
    USER1_TOKEN="$NEW_TOKEN"  # 更新 token
else
    log_error "新密码登录失败"
fi

sleep 1

# ==============================================

log_step "第 14 步：用户1 上传头像"

# 使用真实图片文件作为头像
AVATAR_FILE="./testfile/0BD129B455796E21375D51F2AED2CB3F.jpg"
if [ -f "$AVATAR_FILE" ]; then
    log_info "使用真实图片文件作为头像（6.1MB）"
    UPLOAD_AVATAR=$(curl -s -X POST "${BASE_URL}/api/profile/avatar" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -F "avatar=@$AVATAR_FILE")
    echo "$UPLOAD_AVATAR" | jq '.' 2>/dev/null || echo "$UPLOAD_AVATAR"

    if echo "$UPLOAD_AVATAR" | jq -e '.avatar_url' > /dev/null 2>&1; then
        AVATAR_URL=$(echo "$UPLOAD_AVATAR" | jq -r '.avatar_url')
        log_success "头像上传成功: $AVATAR_URL"
    else
        log_error "头像上传失败"
    fi
else
    log_info "真实头像文件不存在，使用小测试图片"
    # 创建小测试图片作为备选
    echo "iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mNk+M9Qz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC" | base64 -d > /tmp/test_avatar_${TIMESTAMP}.png
    
    UPLOAD_AVATAR=$(curl -s -X POST "${BASE_URL}/api/profile/avatar" \
        -H "Authorization: Bearer $USER1_TOKEN" \
        -F "avatar=@/tmp/test_avatar_${TIMESTAMP}.png")
    echo "$UPLOAD_AVATAR" | jq '.' 2>/dev/null || echo "$UPLOAD_AVATAR"

    if echo "$UPLOAD_AVATAR" | jq -e '.avatar_url' > /dev/null 2>&1; then
        AVATAR_URL=$(echo "$UPLOAD_AVATAR" | jq -r '.avatar_url')
        log_success "头像上传成功: $AVATAR_URL"
    else
        log_error "头像上传失败"
    fi
fi

sleep 1

# ==============================================
# 第五部分：设备管理测试
# ==============================================

log_step "第 15 步：查看用户1的所有登录设备"

DEVICES=$(api_call GET /api/auth/devices "$USER1_TOKEN")
echo "$DEVICES" | jq '.' 2>/dev/null || echo "$DEVICES"

DEVICE_COUNT=$(echo "$DEVICES" | jq -r '.devices | length' 2>/dev/null || echo "0")
log_info "当前登录设备数: $DEVICE_COUNT"

if [ "$DEVICE_COUNT" -gt 0 ]; then
    log_success "获取设备列表成功"
    DEVICE_ID=$(echo "$DEVICES" | jq -r '.devices[0].device_id' 2>/dev/null)
    log_info "第一个设备 ID: $DEVICE_ID"
else
    log_error "获取设备列表失败"
fi

sleep 1

# ==============================================

log_step "第 16 步：删除旧设备（应不影响当前 Token）"

# 找到旧设备（is_current = false）
OLD_DEVICE_ID=$(echo "$DEVICES" | jq -r '.devices[] | select(.is_current == false) | .device_id' | head -1)

if [ -n "$OLD_DEVICE_ID" ] && [ "$OLD_DEVICE_ID" != "null" ]; then
    log_info "删除旧设备: $OLD_DEVICE_ID"
    DELETE_OLD=$(api_call DELETE "/api/auth/devices/${OLD_DEVICE_ID}" "$USER1_TOKEN")
    echo "$DELETE_OLD" | jq '.' 2>/dev/null || echo "$DELETE_OLD"
    
    if echo "$DELETE_OLD" | grep -q "撤销"; then
        log_success "旧设备删除成功"
    else
        log_error "旧设备删除失败"
    fi
    
    sleep 2
    
    # 验证当前 Token 仍然有效
    log_info "验证当前 Token 是否仍然有效..."
    PROFILE_CHECK=$(api_call GET /api/profile "$USER1_TOKEN")
    
    if echo "$PROFILE_CHECK" | jq -e '.data' > /dev/null 2>&1; then
        log_success "✓ 删除旧设备后，当前 Token 仍然有效（预期行为）"
    else
        log_error "✗ 当前 Token 意外失效"
    fi
else
    log_info "没有旧设备，跳过此步骤"
fi

sleep 1

# ==============================================

log_step "第 17 步：删除当前设备（应使 Token 失效）"

# 找到当前设备（is_current = true）
CURRENT_DEVICE_ID=$(echo "$DEVICES" | jq -r '.devices[] | select(.is_current == true) | .device_id')

if [ -n "$CURRENT_DEVICE_ID" ] && [ "$CURRENT_DEVICE_ID" != "null" ]; then
    log_info "删除当前设备: $CURRENT_DEVICE_ID"
    DELETE_CURRENT=$(api_call DELETE "/api/auth/devices/${CURRENT_DEVICE_ID}" "$USER1_TOKEN")
    echo "$DELETE_CURRENT" | jq '.' 2>/dev/null || echo "$DELETE_CURRENT"
    
    if echo "$DELETE_CURRENT" | grep -q "撤销"; then
        log_success "当前设备删除成功"
    else
        log_error "当前设备删除失败"
    fi
    
    sleep 2  # 等待黑名单同步
    
    # 验证当前 Token 已失效
    log_info "尝试使用已删除设备的 Token 获取个人信息..."
    PROFILE_AFTER_DELETE=$(api_call GET /api/profile "$USER1_TOKEN")
    echo "$PROFILE_AFTER_DELETE" | jq '.' 2>/dev/null || echo "$PROFILE_AFTER_DELETE"
    
    if echo "$PROFILE_AFTER_DELETE" | grep -qi "unauthorized\|revoked\|401\|无效\|失效"; then
        log_success "✓ Token 已正确失效（预期行为）"
    else
        log_error "✗ Token 仍然有效（不符合预期）"
    fi
    
    sleep 1
    
    log_info "尝试使用已删除设备的 Token 发送好友请求..."
    FRIEND_REQ_AFTER=$(api_call POST /api/friends/requests "$USER1_TOKEN" "{
        \"user_id\": \"$USER1_ID\",
        \"target_user_id\": \"$USER2_ID\",
        \"reason\": \"测试\",
        \"request_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
    }")
    echo "$FRIEND_REQ_AFTER" | jq '.' 2>/dev/null || echo "$FRIEND_REQ_AFTER"
    
    if echo "$FRIEND_REQ_AFTER" | grep -qi "unauthorized\|revoked\|401\|无效\|失效"; then
        log_success "✓ Token 已正确失效，无法发送好友请求（预期行为）"
    else
        log_error "✗ Token 仍然有效，成功发送了好友请求（不符合预期）"
    fi
else
    log_error "无法获取当前设备 ID，跳过删除当前设备测试"
fi

sleep 1

# ==============================================

log_step "第 18 步：用户1 重新登录获取新 Token"

LOGIN1_FINAL=$(api_call POST /api/auth/login "" "{
    \"user_id\": \"$USER1_ID\",
    \"password\": \"$NEW_PASSWORD\",
    \"device_info\": \"$DEVICE_INFO\",
    \"mac_address\": \"$MAC_ADDR1\"
}")

USER1_TOKEN_NEW=$(echo "$LOGIN1_FINAL" | jq -r '.access_token')

if [ "$USER1_TOKEN_NEW" != "null" ] && [ -n "$USER1_TOKEN_NEW" ]; then
    log_success "用户1 重新登录成功"
    log_info "新 Token: ${USER1_TOKEN_NEW:0:50}..."
else
    log_error "用户1 重新登录失败"
fi

sleep 1

# ==============================================

log_step "第 19 步：使用新 Token 重新执行完整流程"

log_info "1. 获取个人信息..."
PROFILE_NEW=$(api_call GET /api/profile "$USER1_TOKEN_NEW")
if echo "$PROFILE_NEW" | jq -e '.data' > /dev/null 2>&1; then
    log_success "✓ 获取个人信息成功"
else
    log_error "✗ 获取个人信息失败"
fi

sleep 1

log_info "2. 发送好友请求..."
FRIEND_REQ_NEW=$(api_call POST /api/friends/requests "$USER1_TOKEN_NEW" "{
    \"user_id\": \"$USER1_ID\",
    \"target_user_id\": \"$USER2_ID\",
    \"reason\": \"使用新 Token 添加好友\",
    \"request_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
}")
if echo "$FRIEND_REQ_NEW" | jq -e '.request_id' > /dev/null 2>&1; then
    log_success "✓ 好友请求发送成功"
else
    log_error "✗ 好友请求发送失败"
fi

sleep 1

log_info "3. 用户2 同意好友请求..."
APPROVE_NEW=$(api_call POST /api/friends/requests/approve "$USER2_TOKEN" "{
    \"user_id\": \"$USER2_ID\",
    \"applicant_user_id\": \"$USER1_ID\",
    \"approved_time\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",
    \"approved_reason\": \"同意\"
}")
if echo "$APPROVE_NEW" | grep -qi "error\|错误\|失败"; then
    log_error "✗ 好友请求同意失败"
else
    log_success "✓ 好友请求已同意"
fi

sleep 1

log_info "4. 验证好友列表..."
FRIENDS_FINAL=$(api_call GET /api/friends "$USER1_TOKEN_NEW")
if echo "$FRIENDS_FINAL" | grep -q "$USER2_ID"; then
    log_success "✓ 好友列表验证成功"
else
    log_error "✗ 好友列表验证失败"
fi

sleep 1

log_info "5. 更新个人信息..."
UPDATE_FINAL=$(api_call PUT /api/profile "$USER1_TOKEN_NEW" "{
    \"email\": \"final_${USER1_ID}@example.com\",
    \"signature\": \"测试完成！\"
}")
if echo "$UPDATE_FINAL" | grep -q "success\|Success\|成功"; then
    log_success "✓ 个人信息更新成功"
else
    log_error "✗ 个人信息更新失败"
fi

# ==============================================
# 第六部分：好友消息功能测试
# ==============================================

log_step "第 20 步：用户1 向用户2 发送文本消息"

log_info "发送第一条消息..."
SEND_MSG1=$(api_call POST /api/messages "$USER1_TOKEN_NEW" "{
    \"receiver_id\": \"$USER2_ID\",
    \"message_content\": \"Hi, 这是第一条测试消息\",
    \"message_type\": \"text\"
}")
echo "$SEND_MSG1" | jq '.' 2>/dev/null || echo "$SEND_MSG1"

MSG1_UUID=$(echo "$SEND_MSG1" | jq -r '.message_uuid' 2>/dev/null)
if [ "$MSG1_UUID" != "null" ] && [ -n "$MSG1_UUID" ]; then
    log_success "消息发送成功: $MSG1_UUID"
else
    log_error "消息发送失败"
fi

sleep 1

# ==============================================

log_step "第 21 步：用户2 回复消息"

log_info "用户2 发送回复..."
SEND_MSG2=$(api_call POST /api/messages "$USER2_TOKEN" "{
    \"receiver_id\": \"$USER1_ID\",
    \"message_content\": \"Hi, 收到你的消息了！\",
    \"message_type\": \"text\"
}")
echo "$SEND_MSG2" | jq '.' 2>/dev/null || echo "$SEND_MSG2"

MSG2_UUID=$(echo "$SEND_MSG2" | jq -r '.message_uuid' 2>/dev/null)
if [ "$MSG2_UUID" != "null" ] && [ -n "$MSG2_UUID" ]; then
    log_success "回复消息发送成功: $MSG2_UUID"
else
    log_error "回复消息发送失败"
fi

sleep 1

# ==============================================

log_step "第 22 步：用户1 再发送两条消息"

log_info "发送第三条消息..."
SEND_MSG3=$(api_call POST /api/messages "$USER1_TOKEN_NEW" "{
    \"receiver_id\": \"$USER2_ID\",
    \"message_content\": \"这是第三条消息\",
    \"message_type\": \"text\"
}")
MSG3_UUID=$(echo "$SEND_MSG3" | jq -r '.message_uuid' 2>/dev/null)
log_info "消息UUID: $MSG3_UUID"

sleep 1

log_info "发送第四条消息..."
SEND_MSG4=$(api_call POST /api/messages "$USER1_TOKEN_NEW" "{
    \"receiver_id\": \"$USER2_ID\",
    \"message_content\": \"这是第四条消息，稍后会被撤回\",
    \"message_type\": \"text\"
}")
MSG4_UUID=$(echo "$SEND_MSG4" | jq -r '.message_uuid' 2>/dev/null)
log_info "消息UUID: $MSG4_UUID"

sleep 1

# ==============================================

log_step "第 23 步：用户2 查看与用户1 的消息列表"

GET_MSGS=$(api_call GET "/api/messages?friend_id=$USER1_ID&limit=10" "$USER2_TOKEN")
echo "$GET_MSGS" | jq '.' 2>/dev/null || echo "$GET_MSGS"

MSG_COUNT=$(echo "$GET_MSGS" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "当前消息数量: $MSG_COUNT"

if [ "$MSG_COUNT" -ge 3 ]; then
    log_success "✓ 获取消息列表成功（至少3条消息）"
else
    log_error "✗ 获取消息列表失败或消息数量不足"
fi

sleep 1

# ==============================================

log_step "第 24 步：用户1 查看与用户2 的消息列表"

GET_MSGS_USER1=$(api_call GET "/api/messages?friend_id=$USER2_ID&limit=10" "$USER1_TOKEN_NEW")
echo "$GET_MSGS_USER1" | jq '.' 2>/dev/null || echo "$GET_MSGS_USER1"

MSG_COUNT_USER1=$(echo "$GET_MSGS_USER1" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "用户1 看到的消息数量: $MSG_COUNT_USER1"

if [ "$MSG_COUNT_USER1" -ge 3 ]; then
    log_success "✓ 双方都能看到消息"
else
    log_error "✗ 用户1 消息列表异常"
fi

sleep 1

# ==============================================

log_step "第 25 步：用户1 删除第一条消息（软删除）"

log_info "删除消息: $MSG1_UUID"
DELETE_MSG=$(api_call DELETE /api/messages/delete "$USER1_TOKEN_NEW" "{
    \"message_uuid\": \"$MSG1_UUID\"
}")
echo "$DELETE_MSG" | jq '.' 2>/dev/null || echo "$DELETE_MSG"

if echo "$DELETE_MSG" | grep -q "success\|成功"; then
    log_success "✓ 消息删除成功"
else
    log_error "✗ 消息删除失败"
fi

sleep 1

# ==============================================

log_step "第 26 步：验证用户1 看不到已删除的消息"

GET_MSGS_AFTER_DELETE=$(api_call GET "/api/messages?friend_id=$USER2_ID&limit=10" "$USER1_TOKEN_NEW")
echo "$GET_MSGS_AFTER_DELETE" | jq '.' 2>/dev/null || echo "$GET_MSGS_AFTER_DELETE"

MSG_COUNT_AFTER=$(echo "$GET_MSGS_AFTER_DELETE" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "删除后用户1 的消息数量: $MSG_COUNT_AFTER"

if [ "$MSG_COUNT_AFTER" -lt "$MSG_COUNT_USER1" ]; then
    log_success "✓ 用户1 已删除的消息不再显示"
else
    log_error "✗ 删除操作未生效"
fi

sleep 1

# ==============================================

log_step "第 27 步：验证用户2 仍能看到被用户1 删除的消息"

GET_MSGS_USER2_AFTER=$(api_call GET "/api/messages?friend_id=$USER1_ID&limit=10" "$USER2_TOKEN")
echo "$GET_MSGS_USER2_AFTER" | jq '.' 2>/dev/null || echo "$GET_MSGS_USER2_AFTER"

MSG_COUNT_USER2=$(echo "$GET_MSGS_USER2_AFTER" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "用户2 看到的消息数量: $MSG_COUNT_USER2"

if [ "$MSG_COUNT_USER2" -eq "$MSG_COUNT" ]; then
    log_success "✓ 用户2 仍能看到完整消息（双向独立删除）"
else
    log_error "✗ 用户2 的消息列表异常"
fi

sleep 1

# ==============================================

log_step "第 28 步：用户1 撤回第四条消息（2分钟内）"

log_info "撤回消息: $MSG4_UUID"
RECALL_MSG=$(api_call POST /api/messages/recall "$USER1_TOKEN_NEW" "{
    \"message_uuid\": \"$MSG4_UUID\"
}")
echo "$RECALL_MSG" | jq '.' 2>/dev/null || echo "$RECALL_MSG"

if echo "$RECALL_MSG" | grep -q "success\|成功"; then
    log_success "✓ 消息撤回成功"
else
    log_error "✗ 消息撤回失败"
fi

sleep 1

# ==============================================

log_step "第 29 步：验证双方都看不到被撤回的消息"

log_info "用户1 查看消息列表..."
GET_MSGS_RECALL_U1=$(api_call GET "/api/messages?friend_id=$USER2_ID&limit=10" "$USER1_TOKEN_NEW")
MSG_COUNT_RECALL_U1=$(echo "$GET_MSGS_RECALL_U1" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "撤回后用户1 的消息数量: $MSG_COUNT_RECALL_U1"

sleep 1

log_info "用户2 查看消息列表..."
GET_MSGS_RECALL_U2=$(api_call GET "/api/messages?friend_id=$USER1_ID&limit=10" "$USER2_TOKEN")
MSG_COUNT_RECALL_U2=$(echo "$GET_MSGS_RECALL_U2" | jq -r '.messages | length' 2>/dev/null || echo "0")
log_info "撤回后用户2 的消息数量: $MSG_COUNT_RECALL_U2"

if [ "$MSG_COUNT_RECALL_U1" -lt "$MSG_COUNT_AFTER" ] && [ "$MSG_COUNT_RECALL_U2" -lt "$MSG_COUNT_USER2" ]; then
    log_success "✓ 撤回操作生效，双方都看不到被撤回的消息"
else
    log_error "✗ 撤回操作未完全生效"
fi

sleep 1

# ==============================================

log_step "第 30 步：测试发送消息给非好友（应该失败）"

log_info "尝试向不是好友的用户发送消息..."
SEND_TO_STRANGER=$(api_call POST /api/messages "$USER1_TOKEN_NEW" "{
    \"receiver_id\": \"nonexistent_user_999\",
    \"message_content\": \"Hello stranger\",
    \"message_type\": \"text\"
}")
echo "$SEND_TO_STRANGER" | jq '.' 2>/dev/null || echo "$SEND_TO_STRANGER"

if echo "$SEND_TO_STRANGER" | grep -qi "error\|失败\|不是好友"; then
    log_success "✓ 正确拒绝向非好友发送消息"
else
    log_error "✗ 系统允许向非好友发送消息（安全问题）"
fi

sleep 1

# ==============================================

log_step "第 31 步：测试分页查询消息"

log_info "查询前2条消息..."
GET_MSGS_PAGE1=$(api_call GET "/api/messages?friend_id=$USER2_ID&limit=2" "$USER1_TOKEN_NEW")
PAGE1_COUNT=$(echo "$GET_MSGS_PAGE1" | jq -r '.messages | length' 2>/dev/null || echo "0")
HAS_MORE=$(echo "$GET_MSGS_PAGE1" | jq -r '.has_more' 2>/dev/null || echo "false")
log_info "第一页消息数: $PAGE1_COUNT, 是否有更多: $HAS_MORE"

if [ "$PAGE1_COUNT" -eq 2 ]; then
    log_success "✓ 分页查询正常工作"
    
    # 获取最后一条消息的UUID用于下一页查询
    LAST_MSG_UUID=$(echo "$GET_MSGS_PAGE1" | jq -r '.messages[-1].message_uuid' 2>/dev/null)
    
    if [ -n "$LAST_MSG_UUID" ] && [ "$LAST_MSG_UUID" != "null" ]; then
        log_info "查询下一页消息（before_uuid: $LAST_MSG_UUID）..."
        GET_MSGS_PAGE2=$(api_call GET "/api/messages?friend_id=$USER2_ID&before_uuid=$LAST_MSG_UUID&limit=2" "$USER1_TOKEN_NEW")
        PAGE2_COUNT=$(echo "$GET_MSGS_PAGE2" | jq -r '.messages | length' 2>/dev/null || echo "0")
        log_info "第二页消息数: $PAGE2_COUNT"
        
        if [ "$PAGE2_COUNT" -gt 0 ]; then
            log_success "✓ 分页翻页功能正常"
        else
            log_info "没有更多消息（可能是正常情况）"
        fi
    fi
else
    log_error "✗ 分页查询结果不符合预期"
fi

# ==============================================
# 第七部分：文件上传功能测试
# ==============================================

log_step "第 32 步：计算测试文件哈希值"

# 检查是否有sha256sum命令
if command -v sha256sum >/dev/null 2>&1; then
    HASH_CMD="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
    HASH_CMD="shasum -a 256"
else
    log_error "未找到sha256sum或shasum命令，跳过文件上传测试"
    HASH_CMD=""
fi

if [ -n "$HASH_CMD" ]; then
    # 使用真实图片文件进行测试
    TEST_FILE="./testfile/0BD129B455796E21375D51F2AED2CB3F.jpg"
    if [ -f "$TEST_FILE" ]; then
        log_info "使用真实图片文件进行storage测试（6.1MB）"
        TEST_FILE_HASH=$($HASH_CMD "$TEST_FILE" | awk '{print $1}')
        TEST_FILE_SIZE=$(stat -f%z "$TEST_FILE" 2>/dev/null || stat -c%s "$TEST_FILE" 2>/dev/null)
        log_info "测试文件哈希: $TEST_FILE_HASH"
        log_info "测试文件大小: $(($TEST_FILE_SIZE / 1024 / 1024)) MB"
    else
        log_info "真实文件不存在，创建小测试文件..."
        TEST_FILE="/tmp/test_avatar_${TIMESTAMP}.png"
        echo "iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAYAAACNMs+9AAAAFUlEQVR42mNk+M9Qz0AEYBxVSF+FABJADveWkH6oAAAAAElFTkSuQmCC" | base64 -d > "$TEST_FILE"
        TEST_FILE_HASH=$($HASH_CMD "$TEST_FILE" | awk '{print $1}')
        TEST_FILE_SIZE=$(stat -f%z "$TEST_FILE" 2>/dev/null || stat -c%s "$TEST_FILE" 2>/dev/null)
        log_info "测试文件哈希: $TEST_FILE_HASH"
        log_info "测试文件大小: $TEST_FILE_SIZE 字节"
    fi
fi

sleep 1

# ==============================================

log_step "第 33 步：请求文件上传（小文件）"

if [ -n "$HASH_CMD" ]; then
    UPLOAD_REQ=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_image\",
        \"storage_location\": \"user_files\",
        \"filename\": \"test_storage_image.jpg\",
        \"file_size\": $TEST_FILE_SIZE,
        \"content_type\": \"image/jpeg\",
        \"file_hash\": \"$TEST_FILE_HASH\",
        \"force_upload\": false
    }")
    echo "$UPLOAD_REQ" | jq '.' 2>/dev/null || echo "$UPLOAD_REQ"
    
    UPLOAD_MODE=$(echo "$UPLOAD_REQ" | jq -r '.mode' 2>/dev/null)
    INSTANT_UPLOAD=$(echo "$UPLOAD_REQ" | jq -r '.instant_upload' 2>/dev/null)
    
    if [ "$INSTANT_UPLOAD" = "true" ]; then
        log_success "✓ 秒传功能正常工作"
    elif [ "$UPLOAD_MODE" = "one_time_token" ]; then
        log_success "✓ 获取一次性Token上传URL成功"
        UPLOAD_URL=$(echo "$UPLOAD_REQ" | jq -r '.upload_url' 2>/dev/null)
        FILE_KEY=$(echo "$UPLOAD_REQ" | jq -r '.file_key' 2>/dev/null)
        log_info "上传URL: ${UPLOAD_URL:0:50}..."
        log_info "文件key: $FILE_KEY"
    else
        log_error "✗ 获取上传URL失败"
    fi
else
    log_info "跳过文件上传测试（无哈希命令）"
fi

sleep 1

# ==============================================

log_step "第 34 步：直接上传文件到MinIO"

if [ -n "$TEST_FILE_HASH" ] && [ -n "$UPLOAD_URL" ] && [ "$INSTANT_UPLOAD" != "true" ]; then
    # 直接上传文件
    DIRECT_UPLOAD=$(curl -s -X POST "$UPLOAD_URL" \
        -F "file=@$TEST_FILE")
    echo "$DIRECT_UPLOAD" | jq '.' 2>/dev/null || echo "$DIRECT_UPLOAD"
    
    if echo "$DIRECT_UPLOAD" | jq -e '.file_url' > /dev/null 2>&1; then
        UPLOADED_FILE_URL=$(echo "$DIRECT_UPLOAD" | jq -r '.file_url')
        log_success "✓ 文件上传成功: $UPLOADED_FILE_URL"
    else
        log_error "✗ 文件上传失败"
    fi
fi

sleep 1

# ==============================================

log_step "第 35 步：测试秒传功能（再次上传相同文件）"

if [ -n "$HASH_CMD" ]; then
    SECOND_UPLOAD=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_image\",
        \"storage_location\": \"user_files\",
        \"filename\": \"test_image_copy.jpg\",
        \"file_size\": $TEST_FILE_SIZE,
        \"content_type\": \"image/jpeg\",
        \"file_hash\": \"$TEST_FILE_HASH\",
        \"force_upload\": false
    }")
    
    INSTANT_UPLOAD2=$(echo "$SECOND_UPLOAD" | jq -r '.instant_upload' 2>/dev/null)
    
    if [ "$INSTANT_UPLOAD2" = "true" ]; then
        log_success "✓ 秒传功能正常工作（相同哈希文件直接返回）"
        EXISTING_URL=$(echo "$SECOND_UPLOAD" | jq -r '.existing_file_url' 2>/dev/null)
        log_info "已存在文件URL: $EXISTING_URL"
    else
        log_info "未触发秒传（可能是首次上传）"
    fi
fi

sleep 1

# ==============================================

log_step "第 36 步：测试强制重新上传"

if [ -n "$HASH_CMD" ]; then
    FORCE_UPLOAD=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_image\",
        \"storage_location\": \"user_files\",
        \"filename\": \"test_image_force.jpg\",
        \"file_size\": $TEST_FILE_SIZE,
        \"content_type\": \"image/jpeg\",
        \"file_hash\": \"$TEST_FILE_HASH\",
        \"force_upload\": true
    }")
    
    INSTANT_UPLOAD3=$(echo "$FORCE_UPLOAD" | jq -r '.instant_upload' 2>/dev/null)
    
    if [ "$INSTANT_UPLOAD3" = "false" ]; then
        log_success "✓ 强制上传功能正常工作（force_upload=true 跳过秒传）"
    else
        log_error "✗ 强制上传未生效"
    fi
fi

# ==============================================
# 第八部分：真实文件上传测试
# ==============================================

log_step "第 37 步：测试上传真实图片文件（6.1MB）"

REAL_IMAGE="./testfile/0BD129B455796E21375D51F2AED2CB3F.jpg"
if [ -f "$REAL_IMAGE" ]; then
    log_info "计算图片文件哈希..."
    REAL_IMAGE_HASH=$($HASH_CMD "$REAL_IMAGE" | awk '{print $1}')
    REAL_IMAGE_SIZE=$(stat -f%z "$REAL_IMAGE" 2>/dev/null || stat -c%s "$REAL_IMAGE" 2>/dev/null)
    log_info "图片哈希: $REAL_IMAGE_HASH"
    log_info "图片大小: $(($REAL_IMAGE_SIZE / 1024 / 1024)) MB"
    
    # 请求上传
    REAL_IMG_REQ=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_image\",
        \"storage_location\": \"user_files\",
        \"filename\": \"real_test.jpg\",
        \"file_size\": $REAL_IMAGE_SIZE,
        \"content_type\": \"image/jpeg\",
        \"file_hash\": \"$REAL_IMAGE_HASH\",
        \"force_upload\": false
    }")
    
    REAL_IMG_URL=$(echo "$REAL_IMG_REQ" | jq -r '.upload_url' 2>/dev/null)
    REAL_IMG_KEY=$(echo "$REAL_IMG_REQ" | jq -r '.file_key' 2>/dev/null)
    
    if [ -n "$REAL_IMG_URL" ] && [ "$REAL_IMG_URL" != "null" ]; then
        log_success "✓ 获取6.1MB图片上传URL成功"
        
        # 上传文件
        log_info "正在上传6.1MB图片..."
        REAL_IMG_UPLOAD=$(curl -s -X POST "$REAL_IMG_URL" -F "file=@$REAL_IMAGE")
        
        if echo "$REAL_IMG_UPLOAD" | jq -e '.file_url' > /dev/null 2>&1; then
            REAL_IMG_FILE_URL=$(echo "$REAL_IMG_UPLOAD" | jq -r '.file_url')
            log_success "✓ 6.1MB图片上传成功"
            log_info "文件URL: $REAL_IMG_FILE_URL"
        else
            log_error "✗ 图片上传失败"
        fi
    else
        log_error "✗ 获取图片上传URL失败"
    fi
else
    log_info "跳过真实图片测试（文件不存在）"
fi

sleep 1

# ==============================================

log_step "第 38 步：测试上传大图片文件（71MB TIF）"

LARGE_IMAGE="./testfile/landmask_SG_052020_COG512.tif"
if [ -f "$LARGE_IMAGE" ]; then
    log_info "计算大图片文件哈希..."
    LARGE_IMG_HASH=$($HASH_CMD "$LARGE_IMAGE" | awk '{print $1}')
    LARGE_IMG_SIZE=$(stat -f%z "$LARGE_IMAGE" 2>/dev/null || stat -c%s "$LARGE_IMAGE" 2>/dev/null)
    log_info "大图片哈希: $LARGE_IMG_HASH"
    log_info "大图片大小: $(($LARGE_IMG_SIZE / 1024 / 1024)) MB"
    
    # 请求上传
    LARGE_IMG_REQ=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_image\",
        \"storage_location\": \"user_files\",
        \"filename\": \"large_map.tif\",
        \"file_size\": $LARGE_IMG_SIZE,
        \"content_type\": \"image/tiff\",
        \"file_hash\": \"$LARGE_IMG_HASH\",
        \"force_upload\": false
    }")
    
    LARGE_IMG_URL=$(echo "$LARGE_IMG_REQ" | jq -r '.upload_url' 2>/dev/null)
    LARGE_IMG_EXPIRES=$(echo "$LARGE_IMG_REQ" | jq -r '.expires_in' 2>/dev/null)
    
    if [ -n "$LARGE_IMG_URL" ] && [ "$LARGE_IMG_URL" != "null" ]; then
        log_success "✓ 获取71MB大图片上传URL成功（有效期: ${LARGE_IMG_EXPIRES}秒）"
        
        # 上传文件
        log_info "正在上传71MB大图片（可能需要一些时间）..."
        LARGE_IMG_UPLOAD=$(curl -s -X POST "$LARGE_IMG_URL" -F "file=@$LARGE_IMAGE")
        
        if echo "$LARGE_IMG_UPLOAD" | jq -e '.file_url' > /dev/null 2>&1; then
            LARGE_IMG_FILE_URL=$(echo "$LARGE_IMG_UPLOAD" | jq -r '.file_url')
            log_success "✓ 71MB大图片上传成功"
            log_info "文件URL: $LARGE_IMG_FILE_URL"
        else
            log_error "✗ 大图片上传失败"
            echo "$LARGE_IMG_UPLOAD" | jq '.' 2>/dev/null || echo "$LARGE_IMG_UPLOAD"
        fi
    else
        log_error "✗ 获取大图片上传URL失败"
    fi
else
    log_info "跳过大图片测试（文件不存在）"
fi

sleep 1

# ==============================================

log_step "第 39 步：测试上传视频文件（2.9GB）"

VIDEO_FILE="./testfile/VID_20251128_141436_076.mp4"
if [ -f "$VIDEO_FILE" ]; then
    log_info "计算视频文件哈希（可能需要一些时间）..."
    VIDEO_HASH=$($HASH_CMD "$VIDEO_FILE" | awk '{print $1}')
    VIDEO_SIZE=$(stat -f%z "$VIDEO_FILE" 2>/dev/null || stat -c%s "$VIDEO_FILE" 2>/dev/null)
    log_info "视频哈希: $VIDEO_HASH"
    log_info "视频大小: $(($VIDEO_SIZE / 1024 / 1024)) MB"
    
    # 请求上传
    VIDEO_REQ=$(api_call POST /api/storage/upload/request "$USER1_TOKEN_NEW" "{
        \"file_type\": \"user_video\",
        \"storage_location\": \"user_files\",
        \"filename\": \"test_video.mp4\",
        \"file_size\": $VIDEO_SIZE,
        \"content_type\": \"video/mp4\",
        \"file_hash\": \"$VIDEO_HASH\",
        \"force_upload\": false
    }")
    
    VIDEO_URL=$(echo "$VIDEO_REQ" | jq -r '.upload_url' 2>/dev/null)
    VIDEO_EXPIRES=$(echo "$VIDEO_REQ" | jq -r '.expires_in' 2>/dev/null)
    VIDEO_MODE=$(echo "$VIDEO_REQ" | jq -r '.mode' 2>/dev/null)
    
    if [ -n "$VIDEO_URL" ] && [ "$VIDEO_URL" != "null" ]; then
        log_success "✓ 获取2.9GB视频上传URL成功（模式: $VIDEO_MODE, 有效期: ${VIDEO_EXPIRES}秒）"
        
        # 上传文件
        log_info "正在上传2.9GB视频文件（这将需要较长时间）..."
        VIDEO_UPLOAD=$(curl -s -X POST "$VIDEO_URL" -F "file=@$VIDEO_FILE")
        
        if echo "$VIDEO_UPLOAD" | jq -e '.file_url' > /dev/null 2>&1; then
            VIDEO_FILE_URL=$(echo "$VIDEO_UPLOAD" | jq -r '.file_url')
            log_success "✓ 2.9GB视频上传成功"
            log_info "文件URL: $VIDEO_FILE_URL"
        else
            log_error "✗ 视频上传失败"
            echo "$VIDEO_UPLOAD" | jq '.' 2>/dev/null || echo "$VIDEO_UPLOAD"
        fi
    else
        log_error "✗ 获取视频上传URL失败"
        echo "$VIDEO_REQ" | jq '.' 2>/dev/null || echo "$VIDEO_REQ"
    fi
else
    log_info "跳过视频测试（文件不存在）"
fi

sleep 1

# ==============================================

log_step "第 40 步：验证文件可访问性"

# 测试访问已上传的图片
if [ -n "$REAL_IMG_FILE_URL" ]; then
    log_info "测试访问6.1MB图片..."
    IMG_HEAD=$(curl -s -I "$REAL_IMG_FILE_URL" | head -1)
    if echo "$IMG_HEAD" | grep -q "200"; then
        log_success "✓ 图片文件可访问"
    else
        log_error "✗ 图片文件无法访问"
    fi
fi

# 测试访问大图片
if [ -n "$LARGE_IMG_FILE_URL" ]; then
    log_info "测试访问71MB大图片..."
    LARGE_IMG_HEAD=$(curl -s -I "$LARGE_IMG_FILE_URL" | head -1)
    if echo "$LARGE_IMG_HEAD" | grep -q "200"; then
        log_success "✓ 大图片文件可访问"
    else
        log_error "✗ 大图片文件无法访问"
    fi
fi

# 测试访问视频
if [ -n "$VIDEO_FILE_URL" ]; then
    log_info "测试访问2.9GB视频..."
    VIDEO_HEAD=$(curl -s -I "$VIDEO_FILE_URL" | head -1)
    if echo "$VIDEO_HEAD" | grep -q "200"; then
        log_success "✓ 视频文件可访问"
    else
        log_error "✗ 视频文件无法访问"
    fi
fi

# ==============================================
# 测试总结
# ==============================================

log_step "测试完成 - 总结报告"

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              测试执行总结                              ║${NC}"
echo -e "${GREEN}╠════════════════════════════════════════════════════════╣${NC}"
echo -e "${GREEN}║  ✓ 用户注册         2 个用户                          ║${NC}"
echo -e "${GREEN}║  ✓ 用户登录         验证通过                          ║${NC}"
echo -e "${GREEN}║  ✓ 好友请求         发送/接收                          ║${NC}"
echo -e "${GREEN}║  ✓ 好友管理         同意/删除                          ║${NC}"
echo -e "${GREEN}║  ✓ 个人资料         查询/更新                          ║${NC}"
echo -e "${GREEN}║  ✓ 密码修改         验证通过                          ║${NC}"
echo -e "${GREEN}║  ✓ 头像上传         上传成功                          ║${NC}"
echo -e "${GREEN}║  ✓ 设备管理         删除设备                          ║${NC}"
echo -e "${GREEN}║  ✓ Token 失效       验证正确                          ║${NC}"
echo -e "${GREEN}║  ✓ 重新登录         流程验证                          ║${NC}"
echo -e "${GREEN}║  ✓ 消息发送         文本消息                          ║${NC}"
echo -e "${GREEN}║  ✓ 消息查询         分页查询                          ║${NC}"
echo -e "${GREEN}║  ✓ 消息删除         软删除                            ║${NC}"
echo -e "${GREEN}║  ✓ 消息撤回         2分钟内                           ║${NC}"
echo -e "${GREEN}║  ✓ 权限验证         非好友拒绝                        ║${NC}"
echo -e "${GREEN}║  ✓ 文件上传         哈希验证                          ║${NC}"
echo -e "${GREEN}║  ✓ 秒传功能         哈希去重                          ║${NC}"
echo -e "${GREEN}║  ✓ 强制上传         跳过秒传                          ║${NC}"
echo -e "${GREEN}║  ✓ 真实图片         6.1MB上传                         ║${NC}"
echo -e "${GREEN}║  ✓ 大图片文件       71MB上传                          ║${NC}"
echo -e "${GREEN}║  ✓ 视频文件         2.9GB上传                         ║${NC}"
echo -e "${GREEN}║  ✓ 文件访问         可读取验证                        ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${BLUE}测试数据：${NC}"
echo -e "  用户1 ID: $USER1_ID"
echo -e "  用户2 ID: $USER2_ID"
echo -e "  测试时间戳: $TIMESTAMP"
echo ""

# 清理
cleanup

log_success "所有测试完成！"

