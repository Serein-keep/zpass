use crate::crypto::CryptoKey;
use crate::db::DbState;
use std::sync::Mutex;

/// 全局应用状态：包含数据库连接与（内存中的）加密密钥
pub struct AppState {
    pub db: DbState,
    /// 内存中的解密密钥，锁屏时为 None
    pub crypto_key: Mutex<Option<CryptoKey>>,
    /// 上次活动时间（UNIX 秒），用于自动锁屏
    pub last_activity: Mutex<u64>,
}

impl AppState {
    pub fn new(db: DbState) -> Self {
        AppState {
            db,
            crypto_key: Mutex::new(None),
            last_activity: Mutex::new(now_secs()),
        }
    }

    /// 设置密钥并刷新活动时间
    pub fn set_key(&self, key: CryptoKey) {
        *self.crypto_key.lock().unwrap() = Some(key);
        self.touch();
    }

    /// 清除密钥（锁屏）
    pub fn clear_key(&self) {
        *self.crypto_key.lock().unwrap() = None;
    }

    pub fn get_key(&self) -> Option<CryptoKey> {
        self.crypto_key.lock().unwrap().clone()
    }

    /// 刷新活动时间
    pub fn touch(&self) {
        *self.last_activity.lock().unwrap() = now_secs();
    }

    pub fn get_last_activity(&self) -> u64 {
        *self.last_activity.lock().unwrap()
    }
}

pub fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
