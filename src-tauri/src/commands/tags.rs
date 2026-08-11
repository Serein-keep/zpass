use super::{Result, ensure_db_unlocked, upsert_tag};
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_tags(state: State<AppState>) -> Result<Vec<TagOut>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let tags = conn
        .prepare("SELECT id, name, color FROM tags ORDER BY name")
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok(TagOut {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tags)
}

#[tauri::command]
pub fn create_tag(state: State<AppState>, name: String, color: Option<String>) -> Result<TagOut> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let id = upsert_tag(&conn, &name)?;
    let tag = conn
        .query_row(
            "SELECT id, name, color FROM tags WHERE id=?1",
            params![id],
            |r| {
                Ok(TagOut {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;
    if let Some(c) = color {
        conn.execute("UPDATE tags SET color=?2 WHERE id=?1", params![id, c])
            .ok();
    }
    Ok(tag)
}

#[tauri::command]
pub fn delete_tag(state: State<AppState>, name: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute("DELETE FROM entry_tags WHERE tag_id=(SELECT id FROM tags WHERE name=?1)", params![name])
        .ok();
    conn.execute("DELETE FROM tags WHERE name=?1", params![name])
        .map_err(|e| e.to_string())?;
    Ok(())
}
