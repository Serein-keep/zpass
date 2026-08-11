use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::RngCore},
};

fn argon2_instance() -> Argon2<'static> {
    let params = Params::new(8192, 1, 1, None).expect("argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// 加密密钥（256-bit），保存在内存中，锁屏时清除
#[derive(Clone)]
pub struct CryptoKey {
    pub key: [u8; 32],
}

impl CryptoKey {
    /// 从主密码 + salt 派生密钥（Argon2id）
    pub fn derive(password: &str, salt: &[u8]) -> Self {
        let argon2 = argon2_instance();
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .expect("key derivation failed");
        CryptoKey { key }
    }
}

/// 生成随机 salt（用于 Argon2）
pub fn generate_salt() -> Vec<u8> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt.to_vec()
}

/// 使用 Argon2 生成主密码哈希（用于验证，存数据库）
pub fn hash_password(password: &str, salt: &[u8]) -> String {
    let argon2 = argon2_instance();
    let salt_string = SaltString::encode_b64(salt).expect("salt encode failed");
    let hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .expect("hash failed")
        .to_string();
    hash
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match argon2::PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    argon2_instance()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// AES-256-GCM 加密，返回 base64(nonce || ciphertext)
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("加密失败: {}", e))?;
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(hex::encode(combined))
}

/// AES-256-GCM 解密，输入 base64(nonce || ciphertext)
pub fn decrypt(key: &[u8; 32], payload: &str) -> Result<Vec<u8>, String> {
    let combined = hex::decode(payload).map_err(|e| format!("解码失败: {}", e))?;
    if combined.len() < 12 {
        return Err("数据格式错误".to_string());
    }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "解密失败（主密码错误或数据损坏）".to_string())?;
    Ok(plaintext)
}
