use super::{Result, ensure_db_unlocked, require_key, upsert_tag};
use crate::crypto::{self, CryptoKey};
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub master_key: Option<ExportMasterKey>,
    pub entries: Vec<ExportEntry>,
    pub tags: Vec<TagOut>,
    pub entry_tags: Vec<ExportEntryTag>,
    pub settings: Vec<ExportSetting>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportMasterKey {
    pub hash: String,
    pub salt: String,
    pub created_at: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportEntry {
    pub id: String,
    pub category: String,
    pub title: String,
    pub icon: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub otp_id: Option<String>,
    #[serde(default)]
    pub otp_mode: Option<String>,
    pub encrypted_data: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportEntryTag {
    pub entry_id: String,
    pub tag_id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExportSetting {
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub fn export_data(state: State<AppState>, dir_path: String) -> Result<String> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();

    let master_key: Option<ExportMasterKey> = conn
        .query_row(
            "SELECT hash, salt, created_at FROM master_key WHERE id = 1",
            [],
            |r| {
                Ok(ExportMasterKey {
                    hash: r.get(0)?,
                    salt: r.get(1)?,
                    created_at: r.get(2)?,
                })
            },
        )
        .ok();

    let mut entries = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, category, title, icon, note, otp_id, otp_mode, encrypted_data, created_at, updated_at, deleted_at FROM entries")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ExportEntry {
                    id: r.get(0)?,
                    category: r.get(1)?,
                    title: r.get(2)?,
                    icon: r.get(3)?,
                    note: r.get(4)?,
                    otp_id: r.get(5)?,
                    otp_mode: r.get(6)?,
                    encrypted_data: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                    deleted_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut tags = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, name, color FROM tags")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TagOut {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            tags.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut entry_tags = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT entry_id, tag_id FROM entry_tags")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ExportEntryTag {
                    entry_id: r.get(0)?,
                    tag_id: r.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            entry_tags.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut settings = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ExportSetting {
                    key: r.get(0)?,
                    value: r.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            settings.push(row.map_err(|e| e.to_string())?);
        }
    }

    drop(conn);

    let export = ExportData {
        version: 1,
        master_key,
        entries,
        tags,
        entry_tags,
        settings,
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("zpass_export_{}.json", ts);
    let path = std::path::Path::new(&dir_path).join(&filename);
    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub imported: u32,
    pub replaced: u32,
}

#[tauri::command]
pub fn import_data(
    state: State<AppState>,
    file_path: String,
    password: String,
) -> Result<ImportResult> {
    let json_str = std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {}", e))?;
    let export: ExportData =
        serde_json::from_str(&json_str).map_err(|e| format!("解析导入文件失败: {}", e))?;

    if export.version != 1 {
        return Err("不支持的导入文件版本".into());
    }

    let imported_master = export
        .master_key
        .ok_or("导入文件中没有主密码信息")?;

    if !crypto::verify_password(&password, &imported_master.hash) {
        return Err("主密码错误".into());
    }

    let salt = hex::decode(&imported_master.salt).map_err(|_| "salt 解析失败".to_string())?;
    let old_key = CryptoKey::derive(&password, &salt);

    let has_master = {
        let conn = state.db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM master_key", [], |r| r.get(0))
            .unwrap_or(0);
        count > 0
    };

    if has_master {
        let current_key = require_key(&state)?;
        let conn = state.db.conn.lock().unwrap();
        let mut replaced: u32 = 0;

        for entry in &export.entries {
            let existing_id: std::result::Result<String, _> = conn.query_row(
                "SELECT id FROM entries WHERE title=?1 AND deleted_at IS NULL",
                params![entry.title],
                |r| r.get(0),
            );
            if let Ok(old_id) = existing_id {
                conn.execute("DELETE FROM entry_tags WHERE entry_id=?1", params![old_id])
                    .map_err(|e| e.to_string())?;
                conn.execute("DELETE FROM entries WHERE id=?1", params![old_id])
                    .map_err(|e| e.to_string())?;
                replaced += 1;
            }

            let new_id = uuid::Uuid::new_v4().to_string();
            let decrypted = crypto::decrypt(&old_key.key, &entry.encrypted_data)?;
            let re_encrypted = crypto::encrypt(&current_key.key, &decrypted)?;

            conn.execute(
                "INSERT INTO entries (id, category, title, icon, note, otp_id, otp_mode, encrypted_data, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    new_id,
                    entry.category,
                    entry.title,
                    entry.icon,
                    entry.note,
                    entry.otp_id,
                    entry.otp_mode,
                    re_encrypted,
                    entry.created_at,
                    entry.updated_at,
                    entry.deleted_at
                ],
            )
            .map_err(|e| e.to_string())?;

            for et in export.entry_tags.iter().filter(|et| et.entry_id == entry.id) {
                let tag = export.tags.iter().find(|t| t.id == et.tag_id);
                if let Some(tag) = tag {
                    let tag_id = upsert_tag(&conn, &tag.name)?;
                    conn.execute(
                        "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                        params![new_id, tag_id],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(ImportResult {
            imported: export.entries.len() as u32,
            replaced,
        })
    } else {
        let conn = state.db.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO master_key (id, hash, salt, created_at) VALUES (1, ?1, ?2, ?3)",
            params![
                imported_master.hash,
                imported_master.salt,
                imported_master.created_at
            ],
        )
        .map_err(|e| e.to_string())?;

        let salt = hex::decode(&imported_master.salt).map_err(|_| "salt 解析失败".to_string())?;
        let key = CryptoKey::derive(&password, &salt);
        state.set_key(key);

        for entry in &export.entries {
            conn.execute(
                "INSERT OR IGNORE INTO entries (id, category, title, icon, note, otp_id, otp_mode, encrypted_data, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    entry.id,
                    entry.category,
                    entry.title,
                    entry.icon,
                    entry.note,
                    entry.otp_id,
                    entry.otp_mode,
                    entry.encrypted_data,
                    entry.created_at,
                    entry.updated_at,
                    entry.deleted_at
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        for tag in &export.tags {
            conn.execute(
                "INSERT OR IGNORE INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
                params![tag.id, tag.name, tag.color],
            )
            .map_err(|e| e.to_string())?;
        }

        for et in &export.entry_tags {
            conn.execute(
                "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                params![et.entry_id, et.tag_id],
            )
            .map_err(|e| e.to_string())?;
        }

        for s in &export.settings {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
                params![s.key, s.value],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(ImportResult {
            imported: export.entries.len() as u32,
            replaced: 0,
        })
    }
}
