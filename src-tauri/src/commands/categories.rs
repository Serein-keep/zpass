use super::{Result, ensure_db_unlocked};
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Result<Vec<Category>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    list_categories_db(&conn)
}

pub fn list_categories_db(conn: &rusqlite::Connection) -> Result<Vec<Category>> {
    let cats = conn
        .prepare("SELECT id, parent_id, name, icon, color, sort_order, created_at FROM categories ORDER BY sort_order, created_at")
        .map_err(|e| e.to_string())?
        .query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                parent_id: r.get(1)?,
                name: r.get(2)?,
                icon: r.get(3)?,
                color: r.get(4)?,
                sort_order: r.get(5)?,
                created_at: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(cats)
}

#[tauri::command]
pub fn create_category(state: State<AppState>, input: CategoryInput) -> Result<Category> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    create_category_db(&conn, input)
}

pub fn create_category_db(conn: &rusqlite::Connection, input: CategoryInput) -> Result<Category> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("类别名称不能为空".into());
    }
    if let Some(p) = &input.parent_id {
        let exists: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories WHERE id=?1", params![p], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if exists == 0 {
            return Err("父类别不存在".into());
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::state::now_secs().to_string();
    let sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE parent_id IS ?1",
            params![input.parent_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO categories (id, parent_id, name, icon, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, input.parent_id, name, input.icon, input.color, sort, now],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "类别已存在".to_string()
        } else {
            format!("创建类别失败: {}", e)
        }
    })?;
    Ok(Category {
        id,
        parent_id: input.parent_id,
        name,
        icon: input.icon,
        color: input.color,
        sort_order: sort,
        created_at: now,
    })
}

#[tauri::command]
pub fn update_category(
    state: State<AppState>,
    id: String,
    input: CategoryInput,
) -> Result<Category> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    update_category_db(&conn, id, input)
}

pub fn update_category_db(
    conn: &rusqlite::Connection,
    id: String,
    input: CategoryInput,
) -> Result<Category> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("类别名称不能为空".into());
    }
    if input.parent_id.as_deref() == Some(id.as_str()) {
        return Err("类别不能作为自身的父类".into());
    }
    conn.execute(
        "UPDATE categories SET name=?2, parent_id=?3, icon=?4, color=?5 WHERE id=?1",
        params![id, name, input.parent_id, input.icon, input.color],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "类别已存在".to_string()
        } else {
            format!("更新类别失败: {}", e)
        }
    })?;
    load_category(conn, &id)
}

fn load_category(conn: &rusqlite::Connection, id: &str) -> Result<Category> {
    conn.query_row(
        "SELECT id, parent_id, name, icon, color, sort_order, created_at FROM categories WHERE id=?1",
        params![id],
        |r| {
            Ok(Category {
                id: r.get(0)?,
                parent_id: r.get(1)?,
                name: r.get(2)?,
                icon: r.get(3)?,
                color: r.get(4)?,
                sort_order: r.get(5)?,
                created_at: r.get(6)?,
            })
        },
    )
    .map_err(|_| "类别不存在".to_string())
}

#[tauri::command]
pub fn delete_category(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    delete_category_db(&conn, id)
}

pub fn delete_category_db(conn: &rusqlite::Connection, id: String) -> Result<()> {
    let child_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE parent_id=?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if child_count > 0 {
        return Err("该类别下仍有子类别，请先删除".into());
    }
    let tmpl_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM templates WHERE category_id=?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if tmpl_count > 0 {
        return Err("该类别下仍有模板，请先移动或删除".into());
    }
    let entry_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entries WHERE category=?1 AND deleted_at IS NULL",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if entry_count > 0 {
        return Err("该类别下仍有密码条目，无法删除".into());
    }
    conn.execute("DELETE FROM categories WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
