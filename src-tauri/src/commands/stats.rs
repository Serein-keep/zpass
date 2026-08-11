use super::{Result, ensure_db_unlocked};
use crate::models::*;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_category_stats(state: State<AppState>) -> Result<Vec<CategoryStat>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let stats = conn
        .prepare(
            "SELECT category, COUNT(*) FROM entries WHERE deleted_at IS NULL GROUP BY category",
        )
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok(CategoryStat {
                category: r.get(0)?,
                count: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(stats)
}

#[tauri::command]
pub fn get_tag_stats(state: State<AppState>) -> Result<Vec<TagStat>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let stats = conn
        .prepare(
            "SELECT t.id, t.name, t.color, COUNT(et.entry_id) FROM tags t LEFT JOIN entry_tags et ON t.id=et.tag_id LEFT JOIN entries e ON et.entry_id=e.id AND e.deleted_at IS NULL GROUP BY t.id ORDER BY t.name",
        )
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok(TagStat {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                count: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(stats)
}
