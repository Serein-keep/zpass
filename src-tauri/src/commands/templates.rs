use super::{Result, ensure_db_unlocked};
use crate::db::DbState;
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_templates(state: State<AppState>) -> Result<Vec<Template>> {
    ensure_db_unlocked(&state)?;
    list_templates_db(&state.db)
}

pub fn list_templates_db(db: &DbState) -> Result<Vec<Template>> {
    let conn = db.conn.lock().unwrap();
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM templates ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get::<usize, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    drop(conn);
    let mut out = vec![];
    for id in ids {
        out.push(load_template(db, &id)?);
    }
    Ok(out)
}

#[tauri::command]
pub fn get_template(state: State<AppState>, id: String) -> Result<Template> {
    ensure_db_unlocked(&state)?;
    load_template(&state.db, &id)
}

#[tauri::command]
pub fn create_template(state: State<AppState>, input: TemplateInput) -> Result<Template> {
    ensure_db_unlocked(&state)?;
    create_template_db(&state.db, input)
}

pub fn create_template_db(db: &DbState, input: TemplateInput) -> Result<Template> {
    let conn = db.conn.lock().unwrap();
    validate_template_input(&conn, &input)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::state::now_secs().to_string();
    conn.execute(
        "INSERT INTO templates (id, category_id, name, icon, note, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, input.category_id, input.name, input.icon, input.note, now],
    )
    .map_err(|e| format!("创建模板失败: {}", e))?;
    for (idx, f) in input.fields.iter().enumerate() {
        insert_template_field(&conn, &id, f, idx)?;
    }
    drop(conn);
    load_template(db, &id)
}

#[tauri::command]
pub fn update_template(
    state: State<AppState>,
    id: String,
    input: TemplateInput,
) -> Result<Template> {
    ensure_db_unlocked(&state)?;
    update_template_db(&state.db, id, input)
}

pub fn update_template_db(db: &DbState, id: String, input: TemplateInput) -> Result<Template> {
    if id.starts_with("tmpl-") {
        return Err("内置模板不可编辑".into());
    }
    let conn = db.conn.lock().unwrap();
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM templates WHERE id=?1", params![id], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err("模板不存在".into());
    }
    validate_template_input(&conn, &input)?;
    let now = crate::state::now_secs().to_string();
    conn.execute(
        "UPDATE templates SET category_id=?2, name=?3, icon=?4, note=?5, updated_at=?6 WHERE id=?1",
        params![id, input.category_id, input.name, input.icon, input.note, now],
    )
    .map_err(|e| format!("更新模板失败: {}", e))?;
    conn.execute("DELETE FROM template_fields WHERE template_id=?1", params![id])
        .map_err(|e| e.to_string())?;
    for (idx, f) in input.fields.iter().enumerate() {
        insert_template_field(&conn, &id, f, idx)?;
    }
    drop(conn);
    load_template(db, &id)
}

#[tauri::command]
pub fn delete_template(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    delete_template_db(&conn, id)
}

pub fn delete_template_db(conn: &rusqlite::Connection, id: String) -> Result<()> {
    if id.starts_with("tmpl-") {
        return Err("内置模板不可删除".into());
    }
    conn.execute("DELETE FROM template_fields WHERE template_id=?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM templates WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn insert_template_field(
    conn: &rusqlite::Connection,
    template_id: &str,
    f: &TemplateFieldInput,
    sort_order: usize,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO template_fields (id, template_id, name, field_type, secret, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, template_id, f.name, f.field_type, f.secret, sort_order as i64],
    )
    .map_err(|e| format!("保存模板字段失败: {}", e))?;
    Ok(())
}

fn validate_template_input(conn: &rusqlite::Connection, input: &TemplateInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err("模板名称不能为空".into());
    }
    let cat_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE id=?1",
            params![input.category_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if cat_count == 0 {
        return Err("模板类别不存在".into());
    }
    if input.fields.is_empty() {
        return Err("模板至少需要一个字段".into());
    }
    let mut seen = std::collections::HashSet::new();
    for f in &input.fields {
        if f.name.trim().is_empty() {
            return Err("字段名称不能为空".into());
        }
        if !seen.insert(f.name.trim()) {
            return Err(format!("字段名重复: {}", f.name));
        }
        if !crate::models::validate_field_type(&f.field_type) {
            return Err(format!("不支持的字段类型: {}", f.field_type));
        }
    }
    Ok(())
}

pub fn load_template(db: &DbState, id: &str) -> Result<Template> {
    let conn = db.conn.lock().unwrap();
    let row: std::result::Result<(String, String, String, String, Option<String>, Option<String>, String, String), _> = conn.query_row(
        "SELECT t.id, t.category_id, c.name, t.name, t.icon, t.note, t.created_at, t.updated_at FROM templates t JOIN categories c ON t.category_id = c.id WHERE t.id=?1",
        params![id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    );
    let (tid, category_id, category_name, name, icon, note, created_at, updated_at) =
        row.map_err(|_| "模板不存在".to_string())?;
    let fields = conn
        .prepare("SELECT name, field_type, secret FROM template_fields WHERE template_id=?1 ORDER BY sort_order")
        .map_err(|e| e.to_string())?
        .query_map(params![id], |r| {
            Ok(TemplateField {
                name: r.get(0)?,
                field_type: r.get(1)?,
                secret: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Template {
        id: tid.clone(),
        category_id,
        category_name,
        name,
        icon,
        note,
        fields,
        created_at,
        updated_at,
        is_builtin: tid.starts_with("tmpl-"),
    })
}
