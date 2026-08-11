use super::{Result, ensure_db_unlocked, require_key, upsert_tag};
use crate::crypto;
use crate::crypto::CryptoKey;
use crate::db::DbState;
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn create_entry(state: State<AppState>, input: EntryInput) -> Result<EntryOut> {
    ensure_db_unlocked(&state)?;
    let key = require_key(&state)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::state::now_secs().to_string();
    let data_json = serde_json::to_vec(&input.fields).map_err(|e| e.to_string())?;
    let encrypted = crypto::encrypt(&key.key, &data_json)?;

    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO entries (id, category, title, icon, note, otp_id, otp_mode, encrypted_data, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,NULL)",
        params![id, input.category, input.title, input.icon, input.note, input.otp_id, input.otp_mode, encrypted, now, now],
    )
    .map_err(|e| e.to_string())?;

    for tag_name in &input.tags {
        let tag_id = upsert_tag(&conn, tag_name)?;
        conn.execute(
            "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    drop(conn);
    get_entry(state.clone(), id)
}

#[tauri::command]
pub fn update_entry(
    state: State<AppState>,
    id: String,
    input: EntryInput,
) -> Result<EntryOut> {
    ensure_db_unlocked(&state)?;
    let key = require_key(&state)?;
    let now = crate::state::now_secs().to_string();
    let data_json = serde_json::to_vec(&input.fields).map_err(|e| e.to_string())?;
    let encrypted = crypto::encrypt(&key.key, &data_json)?;

    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "UPDATE entries SET category=?2, title=?3, icon=?4, note=?5, otp_id=?6, otp_mode=?7, encrypted_data=?8, updated_at=?9 WHERE id=?1",
        params![id, input.category, input.title, input.icon, input.note, input.otp_id, input.otp_mode, encrypted, now],
    )
    .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM entry_tags WHERE entry_id=?1", params![id])
        .map_err(|e| e.to_string())?;
    for tag_name in &input.tags {
        let tag_id = upsert_tag(&conn, tag_name)?;
        conn.execute(
            "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }
    drop(conn);
    get_entry(state.clone(), id)
}

#[tauri::command]
pub fn get_entries(state: State<AppState>, include_deleted: bool) -> Result<Vec<EntryOut>> {
    ensure_db_unlocked(&state)?;
    let key = require_key(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let sql = if include_deleted {
        "SELECT id FROM entries ORDER BY updated_at DESC"
    } else {
        "SELECT id FROM entries WHERE deleted_at IS NULL ORDER BY updated_at DESC"
    };
    let ids: Vec<String> = conn
        .prepare(sql)
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get::<usize, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    drop(conn);
    let mut out = vec![];
    for id in ids {
        out.push(decode_entry(&state.db, &key, &id)?);
    }
    Ok(out)
}

#[tauri::command]
pub fn get_entry(state: State<AppState>, id: String) -> Result<EntryOut> {
    ensure_db_unlocked(&state)?;
    let key = require_key(&state)?;
    decode_entry(&state.db, &key, &id)
}

/// 软删除（移入回收站）
#[tauri::command]
pub fn delete_entry(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let now = crate::state::now_secs().to_string();
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "UPDATE entries SET deleted_at=?2 WHERE id=?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 恢复
#[tauri::command]
pub fn restore_entry(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute("UPDATE entries SET deleted_at=NULL WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 彻底删除
#[tauri::command]
pub fn permanently_delete_entry(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute("DELETE FROM entry_tags WHERE entry_id=?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM entries WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 清空回收站
#[tauri::command]
pub fn empty_trash(state: State<AppState>) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM entries WHERE deleted_at IS NOT NULL")
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get::<usize, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    for id in ids {
        conn.execute("DELETE FROM entry_tags WHERE entry_id=?1", params![id])
            .ok();
        conn.execute("DELETE FROM entries WHERE id=?1", params![id])
            .ok();
    }
    Ok(())
}

pub fn decode_entry(db: &DbState, key: &CryptoKey, id: &str) -> Result<EntryOut> {
    let conn = db.conn.lock().unwrap();
    let row: std::result::Result<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, String, String, Option<String>), _> = conn.query_row(
        "SELECT category, title, icon, note, otp_id, otp_mode, encrypted_data, updated_at, deleted_at FROM entries WHERE id=?1",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
    );
    let (category, title, icon, note, mut otp_id, mut otp_mode, enc, updated_at, deleted_at) =
        row.map_err(|_| "条目不存在".to_string())?;
    let decrypted = crypto::decrypt(&key.key, &enc)?;
    let mut fields: Vec<Field> =
        serde_json::from_slice(&decrypted).map_err(|e| format!("解析字段失败: {}", e))?;

    // 旧版本数据：OTP 信息曾以伪字段形式存入加密数据，迁出到独立列
    if fields.iter().any(|f| f.name == "OTP验证码" || f.name == "OTP验证模式") {
        if otp_id.is_none() {
            otp_id = fields.iter().find(|f| f.name == "OTP验证码").map(|f| f.value.clone());
            otp_mode = fields.iter().find(|f| f.name == "OTP验证模式").map(|f| f.value.clone());
            let stripped: Vec<Field> = fields
                .iter()
                .filter(|f| f.name != "OTP验证码" && f.name != "OTP验证模式")
                .cloned()
                .collect();
            if let Ok(json) = serde_json::to_vec(&stripped) {
                if let Ok(enc) = crypto::encrypt(&key.key, &json) {
                    conn.execute(
                        "UPDATE entries SET otp_id=?2, otp_mode=?3, encrypted_data=?4 WHERE id=?1",
                        params![id, otp_id, otp_mode, enc],
                    )
                    .ok();
                }
            }
            fields = stripped;
        } else {
            fields.retain(|f| f.name != "OTP验证码" && f.name != "OTP验证模式");
        }
    }

    // 标签
    let tags = conn
        .prepare("SELECT t.id, t.name, t.color FROM tags t JOIN entry_tags et ON t.id=et.tag_id WHERE et.entry_id=?1")
        .map_err(|e| e.to_string())?
        .query_map(params![id], |r| {
            Ok(TagOut { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let created_at: String = conn
        .query_row("SELECT created_at FROM entries WHERE id=?1", params![id], |r| r.get(0))
        .unwrap_or_default();

    Ok(EntryOut {
        id: id.to_string(),
        category,
        title,
        icon,
        note,
        otp_id,
        otp_mode,
        fields,
        tags,
        created_at,
        updated_at,
        deleted_at,
    })
}
