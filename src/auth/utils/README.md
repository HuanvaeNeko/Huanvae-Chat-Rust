# Utils 目录

工具函数层，提供可复用的辅助功能。

## 📂 文件说明

### `crypto.rs` (137 行)
**用途**: RSA 密钥管理和加密工具

**主要功能**:

#### 1. **KeyManager 结构体**
**用途**: 管理 RSA 密钥对，用于 JWT 签名和验证

**字段**:
```rust
pub struct KeyManager {
    private_key: EncodingKey,  // JWT签名用私钥
    public_key: DecodingKey,   // JWT验证用公钥
}
```

---

#### 2. **加载或生成密钥** (`load_or_generate`)
**功能**:
- 检查密钥文件是否存在
- 存在：加载现有密钥
- 不存在：生成新密钥并保存

**参数**:
- `private_key_path`: 私钥文件路径（如 `./keys/private_key.pem`）
- `public_key_path`: 公钥文件路径（如 `./keys/public_key.pem`）

**调用时机**:
- 应用启动时，在 `main.rs` 中调用
- 只调用一次

**行为**:
```
如果密钥文件存在：
    ↓
  加载密钥 (PKCS#1 PEM格式)
    ↓
  转换为 DER 格式
    ↓
  返回 KeyManager

如果密钥文件不存在：
    ↓
  生成2048位RSA密钥对
    ↓
  保存为 PKCS#1 PEM 格式
    ↓
  加载并返回 KeyManager
```

**日志输出**:
```
✅ RSA密钥对加载成功
或
🔧 正在生成新的RSA密钥对...
✅ RSA密钥对生成并保存成功
  私钥: ./keys/private_key.pem
  公钥: ./keys/public_key.pem
```

---

#### 3. **加载密钥** (`load_keys`)
**功能**:
- 从PEM文件加载RSA密钥
- 转换为 `jsonwebtoken` 需要的格式

**流程**:
```
1. 读取PEM文件（文本）
2. 解析 PKCS#1 PEM → RsaPrivateKey/RsaPublicKey
3. 转换为 PKCS#1 DER 格式
4. 创建 EncodingKey/DecodingKey
5. 返回 KeyManager
```

**格式**:
- 文件格式: PKCS#1 PEM
- 内部格式: PKCS#1 DER
- `jsonwebtoken` 使用 DER 格式进行签名/验证

---

#### 4. **生成并保存密钥** (`generate_and_save_keys`)
**功能**:
- 使用操作系统随机数生成器 (`OsRng`)
- 生成 2048位 RSA 密钥对
- 保存为 PKCS#1 PEM 格式

**安全性**:
- 使用 `OsRng` (密码学安全的随机数生成器)
- 2048位密钥（足够的安全强度）
- 私钥应妥善保管，不应提交到版本控制

**文件内容示例**:

私钥 (`private_key.pem`):
```
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
-----END RSA PRIVATE KEY-----
```

公钥 (`public_key.pem`):
```
-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEA...
-----END RSA PUBLIC KEY-----
```

---

#### 5. **获取密钥** (公开方法)
```rust
pub fn encoding_key(&self) -> &EncodingKey  // 获取签名用私钥
pub fn decoding_key(&self) -> &DecodingKey  // 获取验证用公钥
```

**调用时机**:
- `TokenService` 中签名 JWT
- `TokenService` 中验证 JWT

---

**依赖**:
- `rsa` crate - RSA密钥生成和操作
- `jsonwebtoken` crate - JWT签名和验证
- `rand_core::OsRng` - 密码学安全随机数

**错误处理**:
- 返回 `Result<KeyManager, AuthError>`
- 错误类型: `AuthError::CryptoError`

---

### `password.rs` (32 行)
**用途**: 密码哈希和验证

**主要功能**:

#### 1. **密码哈希** (`hash_password`)
**功能**:
- 使用 bcrypt 算法加密密码
- 自动生成随机盐值
- 返回哈希字符串

**参数**:
- `password`: &str - 明文密码

**返回**:
- `Result<String, AuthError>` - bcrypt 哈希字符串

**示例**:
```rust
let password = "MySecurePassword123";
let hash = hash_password(password)?;
// hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCO..."
```

**调用时机**:
- 用户注册时，在 `register_handler` 中
- 用户修改密码时

**安全特性**:
- Cost: 12（默认，2^12次迭代）
- 自动随机盐值
- 单向哈希，不可逆

**性能**:
- ~500ms per hash（故意慢，防止暴力破解）
- 建议异步处理，避免阻塞

---

#### 2. **密码验证** (`verify_password`)
**功能**:
- 验证明文密码是否匹配哈希
- 返回布尔值

**参数**:
- `password`: &str - 用户输入的明文密码
- `hash`: &str - 数据库中存储的哈希

**返回**:
- `Result<bool, AuthError>` - 是否匹配

**示例**:
```rust
let input_password = "MySecurePassword123";
let stored_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCO...";

if verify_password(input_password, stored_hash)? {
    println!("密码正确！");
} else {
    println!("密码错误！");
}
```

**调用时机**:
- 用户登录时，在 `login_handler` 中
- 修改密码时验证旧密码

**安全性**:
- 时间恒定比较（防止时序攻击）
- bcrypt 内置防护

---

**依赖**:
- `bcrypt` crate - 密码哈希算法

**为什么选择 bcrypt**:
- ✅ 专为密码设计
- ✅ 自动盐值管理
- ✅ 可调整的计算成本
- ✅ 抗彩虹表攻击
- ✅ 抗GPU破解（内存密集）

---

### `validator.rs` (24 行)
**用途**: 输入验证

**主要功能**:

#### 1. **邮箱验证** (`validate_email`)
**功能**:
- 检查邮箱格式是否合法
- 简单的正则表达式验证

**验证规则**:
```rust
const EMAIL_REGEX: &str = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
```

**规则说明**:
- `[a-zA-Z0-9._%+-]+` - 本地部分（@之前）
- `@` - 必须包含@符号
- `[a-zA-Z0-9.-]+` - 域名部分
- `\.[a-zA-Z]{2,}` - 顶级域名（至少2个字母）

**示例**:
```rust
validate_email("user@example.com")?;     // ✅ 通过
validate_email("user@example.co.uk")?;   // ✅ 通过
validate_email("user@example")?;         // ❌ 失败：缺少TLD
validate_email("user.example.com")?;     // ❌ 失败：缺少@
```

**调用时机**:
- 用户注册时
- 在 `register_handler` 中调用

**返回**:
- `Result<(), AuthError>` - 成功或错误

---

#### 2. **昵称验证** (`validate_nickname`)
**功能**:
- 检查昵称长度

**验证规则**:
- 长度: 2-50 字符

**示例**:
```rust
validate_nickname("张三")?;        // ✅ 通过
validate_nickname("A")?;           // ❌ 失败：太短
validate_nickname("很长的昵称...")?; // ❌ 失败：太长（>50字符）
```

**调用时机**:
- 用户注册时
- 修改昵称时

---

#### 3. **密码强度验证** (`validate_password_strength`)
**功能**:
- 检查密码是否足够强

**验证规则**:
- 最小长度: 8字符
- 必须包含字母
- 必须包含数字

**示例**:
```rust
validate_password_strength("Pass123")?;      // ❌ 失败：太短
validate_password_strength("Password")?;     // ❌ 失败：缺少数字
validate_password_strength("12345678")?;     // ❌ 失败：缺少字母
validate_password_strength("Password123")?;  // ✅ 通过
```

**调用时机**:
- 用户注册时
- 修改密码时
- 在 `register_handler` 中调用

**返回**:
- `Result<(), AuthError>` - 成功或错误

---

**依赖**:
- `regex` crate - 正则表达式验证

**扩展建议**:
未来可以增强验证规则：
- ✨ 密码必须包含特殊字符
- ✨ 检查常见弱密码（如 "password123"）
- ✨ 昵称不允许特殊字符
- ✨ 邮箱域名白名单/黑名单
- ✨ 使用 `validator` crate 的 derive 宏

---

### `mod.rs` (9 行)
**用途**: 模块导出

**导出内容**:
```rust
// crypto.rs
pub use crypto::KeyManager;

// password.rs
pub use password::{hash_password, verify_password};

// validator.rs
pub use validator::{validate_email, validate_nickname, validate_password_strength};
```

---

## 🔄 使用流程

### 应用启动
```rust
// main.rs
let key_manager = KeyManager::load_or_generate(
    "./keys/private_key.pem",
    "./keys/public_key.pem"
)?;
```

### 用户注册
```rust
// register_handler
validate_email(&req.email)?;
validate_nickname(&req.nickname)?;
validate_password_strength(&req.password)?;

let password_hash = hash_password(&req.password)?;
// 保存 password_hash 到数据库
```

### 用户登录
```rust
// login_handler
let user = query_user_by_email(&req.email).await?;

if !verify_password(&req.password, &user.user_password)? {
    return Err(AuthError::InvalidCredentials);
}

// 生成 Token...
```

### JWT 签名
```rust
// token_service.rs
let token = encode(
    &Header::new(Algorithm::RS256),
    &claims,
    key_manager.encoding_key(),  // 使用私钥签名
)?;
```

### JWT 验证
```rust
// token_service.rs
let token_data = decode::<AccessTokenClaims>(
    &token,
    key_manager.decoding_key(),  // 使用公钥验证
    &validation,
)?;
```

---

## 🔒 安全最佳实践

### 密钥管理
- ✅ 私钥绝不提交到版本控制
- ✅ 使用 `.gitignore` 排除 `keys/` 目录
- ✅ 生产环境使用环境变量或密钥管理服务
- ✅ 定期轮换密钥（考虑密钥版本管理）

### 密码安全
- ✅ 使用 bcrypt（或 argon2）
- ✅ Cost factor 至少为 12
- ✅ 从不存储明文密码
- ✅ 从不在日志中记录密码

### 输入验证
- ✅ 在服务端验证所有输入
- ✅ 不信任客户端验证
- ✅ 及时返回明确的错误信息
- ✅ 防止SQL注入（使用参数化查询）

---

## 📊 性能考虑

### Crypto
- ⚡ RSA签名: ~5ms
- ⚡ RSA验证: ~2ms
- ✅ 密钥加载只在启动时执行一次

### Password
- 🐌 bcrypt哈希: ~500ms（故意慢）
- 🐌 bcrypt验证: ~500ms（故意慢）
- ⚠️ 考虑使用异步或后台任务
- ⚠️ 注册/登录接口可能需要更长超时

### Validator
- ⚡ 邮箱验证: <1ms
- ⚡ 昵称验证: <1ms
- ⚡ 密码强度验证: <1ms
- ✅ 几乎无性能影响

---

## 🎯 设计原则

**职责边界**:
- ✅ 提供纯函数工具
- ✅ 无状态（除了 KeyManager）
- ✅ 可独立测试
- ✅ 高度可复用
- ❌ 不包含业务逻辑
- ❌ 不直接操作数据库

**错误处理**:
- 统一返回 `Result<T, AuthError>`
- 详细的错误信息
- 便于上层处理

**可扩展性**:
- 易于添加新的验证规则
- 易于更换加密算法
- 易于添加新的工具函数

