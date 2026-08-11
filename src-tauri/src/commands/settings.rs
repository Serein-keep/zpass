use super::Result;
use crate::models::Settings;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_database_path(state: State<AppState>) -> String {
    let settings = get_settings(state.clone()).unwrap_or(Settings {
        lock_timeout: 30,
        theme: "light".into(),
        storage_path: String::new(),
    });
    if settings.storage_path.is_empty() {
        crate::db::default_db_path().to_string_lossy().to_string()
    } else {
        std::path::Path::new(&settings.storage_path)
            .join("zpass.db")
            .to_string_lossy()
            .to_string()
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    let conn = state.db.conn.lock().unwrap();
    let get = |k: &str| -> String {
        conn.query_row("SELECT value FROM settings WHERE key=?1", params![k], |r| r.get(0))
            .unwrap_or_default()
    };
    Ok(Settings {
        lock_timeout: get("lock_timeout").parse().unwrap_or(30),
        theme: get("theme"),
        storage_path: get("storage_path"),
    })
}

#[tauri::command]
pub fn update_setting(state: State<AppState>, key: String, value: String) -> Result<()> {
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
