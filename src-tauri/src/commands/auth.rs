use super::Result;
use crate::crypto::{self, CryptoKey};
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

/// 检查是否已设置主密码
#[tauri::command]
pub fn has_master_password(state: State<AppState>) -> bool {
    let conn = state.db.conn.lock().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM master_key", [], |r| r.get(0))
        .unwrap_or(0);
    count > 0
}

/// 设置主密码（首次）
#[tauri::command]
pub fn set_master_password(state: State<AppState>, password: String) -> Result<()> {
    if has_master_password(state.clone()) {
        return Err("主密码已设置，不可更改".into());
    }
    let salt = crypto::generate_salt();
    let hash = crypto::hash_password(&password, &salt);
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO master_key (id, hash, salt, created_at) VALUES (1, ?1, ?2, ?3)",
        params![hash, hex::encode(&salt), crate::state::now_secs().to_string()],
    )
    .map_err(|e| format!("保存主密码失败: {}", e))?;
    // 派生密钥并存入内存
    let key = CryptoKey::derive(&password, &salt);
    state.set_key(key);
    Ok(())
}

/// 验证主密码并解锁
#[tauri::command]
pub fn unlock(state: State<AppState>, password: String) -> Result<()> {
    let conn = state.db.conn.lock().unwrap();
    let row: std::result::Result<(String, String), _> = conn.query_row(
        "SELECT hash, salt FROM master_key WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    let (hash, salt_hex) = row.map_err(|_| "尚未设置主密码".to_string())?;
    if !crypto::verify_password(&password, &hash) {
        return Err("主密码错误".into());
    }
    let salt = hex::decode(&salt_hex).map_err(|_| "salt 解析失败".to_string())?;
    let key = CryptoKey::derive(&password, &salt);
    state.set_key(key);
    Ok(())
}

/// 锁屏：清除内存密钥
#[tauri::command]
pub fn lock(state: State<AppState>) {
    state.clear_key();
}

/// 心跳：刷新活动时间
#[tauri::command]
pub fn heartbeat(state: State<AppState>) {
    state.touch();
}
