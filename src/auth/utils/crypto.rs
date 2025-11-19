use crate::auth::errors::AuthError;
use jsonwebtoken::{DecodingKey, EncodingKey};
use rsa::{
    pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, DecodeRsaPrivateKey, DecodeRsaPublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey, RsaPublicKey,
};
use std::fs;
use std::path::Path;

/// RSA 密钥对管理器
pub struct KeyManager {
    private_key: EncodingKey,
    public_key: DecodingKey,
}

impl KeyManager {
    /// 加载或生成 RSA 密钥对
    pub fn load_or_generate(
        private_key_path: &str,
        public_key_path: &str,
    ) -> Result<Self, AuthError> {
        // 检查密钥文件是否存在
        if Path::new(private_key_path).exists() && Path::new(public_key_path).exists() {
            // 加载现有密钥
            Self::load_keys(private_key_path, public_key_path)
        } else {
            // 生成新密钥
            Self::generate_and_save_keys(private_key_path, public_key_path)
        }
    }

    /// 从文件加载密钥
    fn load_keys(private_key_path: &str, public_key_path: &str) -> Result<Self, AuthError> {
        let private_pem = fs::read_to_string(private_key_path)
            .map_err(|e| AuthError::CryptoError(format!("读取私钥失败: {}", e)))?;

        let public_pem = fs::read_to_string(public_key_path)
            .map_err(|e| AuthError::CryptoError(format!("读取公钥失败: {}", e)))?;

        // 解析 PKCS#1 PEM 格式
        let private_key_rsa = RsaPrivateKey::from_pkcs1_pem(&private_pem)
            .map_err(|e| AuthError::CryptoError(format!("解析私钥失败: {}", e)))?;
        
        let public_key_rsa = RsaPublicKey::from_pkcs1_pem(&public_pem)
            .map_err(|e| AuthError::CryptoError(format!("解析公钥失败: {}", e)))?;

        // 转换为 PKCS#1 DER 格式供 jsonwebtoken 使用
        let private_der = private_key_rsa
            .to_pkcs1_der()
            .map_err(|e| AuthError::CryptoError(format!("转换私钥为DER失败: {}", e)))?;
        
        let public_der = public_key_rsa
            .to_pkcs1_der()
            .map_err(|e| AuthError::CryptoError(format!("转换公钥为DER失败: {}", e)))?;

        let private_key = EncodingKey::from_rsa_der(private_der.as_bytes());
        let public_key = DecodingKey::from_rsa_der(public_der.as_bytes());

        tracing::info!("✅ RSA密钥对加载成功");

        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// 生成并保存新密钥
    fn generate_and_save_keys(
        private_key_path: &str,
        public_key_path: &str,
    ) -> Result<Self, AuthError> {
        tracing::info!("🔧 正在生成新的RSA密钥对...");

        // 使用 OsRng (操作系统随机数生成器)，它实现了 CryptoRngCore
        let mut rng = OsRng;
        let bits = 2048;

        // 生成 RSA 私钥
        let private_key_rsa = RsaPrivateKey::new(&mut rng, bits)
            .map_err(|e| AuthError::CryptoError(format!("生成私钥失败: {}", e)))?;

        // 从私钥派生公钥
        let public_key_rsa = RsaPublicKey::from(&private_key_rsa);

        // 序列化为 PKCS#1 PEM 格式
        let private_pem = private_key_rsa
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|e| AuthError::CryptoError(format!("序列化私钥失败: {}", e)))?;

        let public_pem = public_key_rsa
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|e| AuthError::CryptoError(format!("序列化公钥失败: {}", e)))?;

        // 确保目录存在
        if let Some(parent) = Path::new(private_key_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AuthError::CryptoError(format!("创建密钥目录失败: {}", e)))?;
        }

        // 保存到文件
        fs::write(private_key_path, private_pem.as_bytes())
            .map_err(|e| AuthError::CryptoError(format!("保存私钥失败: {}", e)))?;

        fs::write(public_key_path, public_pem.as_bytes())
            .map_err(|e| AuthError::CryptoError(format!("保存公钥失败: {}", e)))?;

        tracing::info!("✅ RSA密钥对生成并保存成功");
        tracing::info!("  私钥: {}", private_key_path);
        tracing::info!("  公钥: {}", public_key_path);

        // 加载刚生成的密钥
        Self::load_keys(private_key_path, public_key_path)
    }

    /// 获取用于签名的私钥
    pub fn encoding_key(&self) -> &EncodingKey {
        &self.private_key
    }

    /// 获取用于验证的公钥
    pub fn decoding_key(&self) -> &DecodingKey {
        &self.public_key
    }
}

/// 生成随机 JWT ID (JTI)
pub fn generate_jti() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 生成设备ID
pub fn generate_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

