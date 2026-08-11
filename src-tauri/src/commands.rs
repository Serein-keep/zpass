use crate::crypto::{self, CryptoKey};
use crate::db::DbState;
use crate::models;
use crate::models::*;
use crate::state::AppState;
use rusqlite::params;
use tauri::State;
use tauri::Manager;

type Result<T> = std::result::Result<T, String>;

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

/// 内部：获取密钥或报错
fn require_key(state: &AppState) -> Result<CryptoKey> {
    state.get_key().ok_or_else(|| "应用已锁定，请先解锁".to_string())
}

fn ensure_db_unlocked(state: &AppState) -> Result<()> {
    if state.get_key().is_none() {
        return Err("应用已锁定".into());
    }
    Ok(())
}

// ---------------- 条目 CRUD ----------------

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

    // 处理标签
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

    // 重建标签
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
    conn.execute("DELETE FROM custom_fields WHERE entry_id=?1", params![id])
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
        conn.execute("DELETE FROM custom_fields WHERE entry_id=?1", params![id])
            .ok();
        conn.execute("DELETE FROM entries WHERE id=?1", params![id])
            .ok();
    }
    Ok(())
}

// ---------------- 标签 ----------------

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
    // 更新颜色
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

// ---------------- 类别（树结构，同时作为模板分类） ----------------

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Result<Vec<Category>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    list_categories_db(&conn)
}

fn list_categories_db(conn: &rusqlite::Connection) -> Result<Vec<Category>> {
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

fn create_category_db(conn: &rusqlite::Connection, input: CategoryInput) -> Result<Category> {
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

fn update_category_db(
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

fn delete_category_db(conn: &rusqlite::Connection, id: String) -> Result<()> {
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

#[tauri::command]
pub fn list_templates(state: State<AppState>) -> Result<Vec<Template>> {
    ensure_db_unlocked(&state)?;
    list_templates_db(&state.db)
}

fn list_templates_db(db: &DbState) -> Result<Vec<Template>> {
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

fn create_template_db(db: &DbState, input: TemplateInput) -> Result<Template> {
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

fn update_template_db(db: &DbState, id: String, input: TemplateInput) -> Result<Template> {
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

fn delete_template_db(conn: &rusqlite::Connection, id: String) -> Result<()> {
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
        if !models::validate_field_type(&f.field_type) {
            return Err(format!("不支持的字段类型: {}", f.field_type));
        }
    }
    Ok(())
}

fn load_template(db: &DbState, id: &str) -> Result<Template> {
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

// ---------------- 统计 ----------------

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

// ---------------- 设置 ----------------

#[tauri::command]
pub fn get_database_path(state: State<AppState>) -> String {
    let settings = get_settings(state.clone()).unwrap_or(models::Settings {
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

// ---------------- 辅助函数 ----------------

fn upsert_tag(conn: &rusqlite::Connection, name: &str) -> Result<String> {
    // 查询是否存在
    let existing: std::result::Result<String, _> = conn.query_row(
        "SELECT id FROM tags WHERE name=?1",
        params![name],
        |r| r.get(0),
    );
    if let Ok(id) = existing {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tags (id, name, color) VALUES (?1, ?2, NULL)",
        params![id, name],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

fn decode_entry(db: &DbState, key: &CryptoKey, id: &str) -> Result<EntryOut> {
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

// ---------------- OTP CRUD ----------------

#[tauri::command]
pub fn get_otp_entries(state: State<AppState>) -> Result<Vec<OtpEntryOut>> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, issuer, account, secret, interval, digits, algorithm, created_at FROM otp_entries ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let entries = stmt
        .query_map([], |r| {
            Ok(OtpEntryOut {
                id: r.get(0)?,
                issuer: r.get(1)?,
                account: r.get(2)?,
                secret: r.get(3)?,
                interval: r.get(4)?,
                digits: r.get(5)?,
                algorithm: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

#[tauri::command]
pub fn create_otp_entry(state: State<AppState>, input: OtpEntryInput) -> Result<OtpEntryOut> {
    ensure_db_unlocked(&state)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::state::now_secs().to_string();
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO otp_entries (id, issuer, account, secret, interval, digits, algorithm, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![id, input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(OtpEntryOut {
        id,
        issuer: input.issuer,
        account: input.account,
        secret: input.secret,
        interval: input.interval,
        digits: input.digits,
        algorithm: input.algorithm,
        created_at: now,
    })
}

#[tauri::command]
pub fn delete_otp_entry(state: State<AppState>, id: String) -> Result<()> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute("DELETE FROM otp_entries WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_otp_entry(state: State<AppState>, id: String, input: OtpEntryInput) -> Result<OtpEntryOut> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    conn.execute(
        "UPDATE otp_entries SET issuer=?1, account=?2, secret=?3, interval=?4, digits=?5, algorithm=?6 WHERE id=?7",
        params![input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, id],
    )
    .map_err(|e| e.to_string())?;
    let row: std::result::Result<(String, String, String, String, u32, u32, String, String), _> = conn.query_row(
        "SELECT id, issuer, account, secret, interval, digits, algorithm, created_at FROM otp_entries WHERE id=?1",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
    );
    row.map(|r| OtpEntryOut {
        id: r.0,
        issuer: r.1,
        account: r.2,
        secret: r.3,
        interval: r.4,
        digits: r.5,
        algorithm: r.6,
        created_at: r.7,
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_otp_entries(state: State<AppState>, entries: Vec<OtpEntryInput>) -> Result<u32> {
    ensure_db_unlocked(&state)?;
    let conn = state.db.conn.lock().unwrap();
    let mut count = 0u32;
    for input in entries {
        let id = uuid::Uuid::new_v4().to_string();
        let now = crate::state::now_secs().to_string();
        conn.execute(
            "INSERT INTO otp_entries (id, issuer, account, secret, interval, digits, algorithm, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, input.issuer, input.account, input.secret, input.interval, input.digits, input.algorithm, now],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
pub async fn capture_qr_code(app: tauri::AppHandle) -> Result<String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.hide().map_err(|e| e.to_string())?;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let result = (|| -> Result<String> {
        let screenshot = screenshots::Screen::all()
            .map_err(|e| format!("截图失败: {}", e))?
            .into_iter()
            .next()
            .ok_or("未找到屏幕")?
            .capture()
            .map_err(|e| format!("截图失败: {}", e))?;
        let width = screenshot.width() as usize;
        let raw = screenshot.into_raw();
        let luma: Vec<u8> = raw.chunks(4).map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        }).collect();
        let threshold = 128u8;
        let grid = rqrr::SimpleGrid::from_func(width, |x, y| {
            let idx = y * width + x;
            idx < luma.len() && luma[idx] < threshold
        });
        let decoded = rqrr::Grid::new(grid).decode().map_err(|_| "未检测到二维码")?;
        Ok(decoded.1)
    })();

    window.show().map_err(|e| e.to_string())?;
    result
}

#[tauri::command]
pub fn read_file_text(path: String) -> Result<String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {}", e))?;
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).to_string()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    Ok(content)
}

#[tauri::command]
pub fn decode_qr_image(path: String) -> Result<String> {
    let img = image::open(&path).map_err(|e| format!("打开图片失败: {}", e))?;
    let rgba = img.to_rgba8();
    let results = bardecoder::default_decoder().decode(&rgba);
    for result in results {
        if let Ok(text) = result {
            return Ok(text);
        }
    }
    Err("未检测到二维码，请确保图片清晰且二维码完整".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::CryptoKey;
    use crate::db::DbState;
    use crate::state::AppState;

    fn temp_db_path() -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let name = format!("zpass_test_{}.db", uuid::Uuid::new_v4());
        dir.join(name)
    }

    /// 构造已解锁的 AppState（临时库 + 固定密钥）
    fn unlocked_state() -> AppState {
        let db = DbState::new(temp_db_path()).expect("初始化测试库失败");
        let state = AppState::new(db);
        state.set_key(CryptoKey::derive("test-password", b"0123456789abcdef"));
        state
    }

    /// 向 entries 插入一条加密数据（复用 create_entry 的加密与列布局），返回条目 id
    fn insert_entry_raw(
        state: &AppState,
        id: &str,
        fields_json: &[u8],
    ) {
        let key = state.get_key().unwrap();
        let enc = crypto::encrypt(&key.key, fields_json).unwrap();
        let now = crate::state::now_secs().to_string();
        let conn = state.db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entries (id, category, title, icon, note, otp_id, otp_mode, encrypted_data, created_at, updated_at, deleted_at) VALUES (?1,?2,?3,NULL,NULL,NULL,NULL,?4,?5,?5,NULL)",
            params![id, "login", "测试条目", enc, now],
        )
        .unwrap();
    }

    #[test]
    fn migration_drops_template_id_and_seeds_builtin() {
        let state = unlocked_state();
        let conn = state.db.conn.lock().unwrap();
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(entries)")
            .unwrap()
            .query_map([], |r| r.get::<usize, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(!cols.contains(&"template_id".to_string()), "entries 表不应含 template_id 列");
        let tmpl_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM templates", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tmpl_count, 4, "应种子 4 个内置模板");
        let field_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_fields", [], |r| r.get(0))
            .unwrap();
        assert_eq!(field_count, 14, "内置模板字段应为 14 行");
        let cat_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cat_count, 4, "应种子 4 个内置类别");
        let login_cat: (Option<String>, String) = conn
            .query_row("SELECT icon, color FROM categories WHERE id='login'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(login_cat.0.as_deref(), Some("globe"), "内置类别应有图标");
        assert!(login_cat.1.starts_with('#'), "内置类别应有颜色");
    }

    #[test]
    fn category_tree_crud_and_guards() {
        let state = unlocked_state();
        let conn = state.db.conn.lock().unwrap();

        // 内置 4 类别存在，可添加子类
        let child = create_category_db(
            &conn,
            CategoryInput {
                name: "公司邮箱".into(),
                parent_id: Some("email".into()),
                icon: Some("mail".into()),
                color: Some("#ff0000".into()),
            },
        )
        .expect("创建子类别失败");
        assert_eq!(child.parent_id.as_deref(), Some("email"));

        let dup = create_category_db(
            &conn,
            CategoryInput {
                name: "公司邮箱".into(),
                parent_id: None,
                icon: None,
                color: None,
            },
        );
        assert!(dup.is_err(), "重名类别应报错");

        let updated = update_category_db(
            &conn,
            child.id.clone(),
            CategoryInput {
                name: "办公邮箱".into(),
                parent_id: Some("email".into()),
                icon: Some("mail".into()),
                color: None,
            },
        )
        .expect("更新类别失败");
        assert_eq!(updated.name, "办公邮箱");

        let self_parent = update_category_db(
            &conn,
            child.id.clone(),
            CategoryInput {
                name: "办公邮箱".into(),
                parent_id: Some(child.id.clone()),
                icon: None,
                color: None,
            },
        );
        assert!(self_parent.is_err(), "类别不能作为自身的父类");

        // 删除保护：有子类别 / 有模板 / 有条目
        let del_child = delete_category_db(&conn, "email".into());
        assert!(del_child.is_err(), "有子类别的类别删除应被阻止");

        let del_with_tmpl = delete_category_db(&conn, "login".into());
        assert!(del_with_tmpl.is_err(), "有模板的类别删除应被阻止");

        let del_with_entry = delete_category_db(&conn, "custom".into());
        assert!(del_with_entry.is_err(), "有模板的类别删除应被阻止");

        // 先删除子类，再删父类（email 有内置模板，删除仍被阻止）
        delete_category_db(&conn, child.id).expect("删除子类别失败");
        let del_parent = delete_category_db(&conn, "email".into());
        assert!(del_parent.is_err(), "有模板的 email 类别删除应被阻止");

        // 新建一个完全空的类别并删除
        let tmp = create_category_db(
            &conn,
            CategoryInput {
                name: "临时类别".into(),
                parent_id: None,
                icon: None,
                color: None,
            },
        )
        .expect("创建临时类别失败");
        delete_category_db(&conn, tmp.id).expect("删除空类别失败");
    }

    #[test]
    fn template_crud_with_fields_and_order() {
        let state = unlocked_state();
        let input = TemplateInput {
            category_id: "custom".into(),
            name: "SSH服务器".into(),
            icon: None,
            note: None,
            fields: vec![
                TemplateFieldInput {
                    name: "主机".into(),
                    field_type: "text".into(),
                    secret: false,
                },
                TemplateFieldInput {
                    name: "密码".into(),
                    field_type: "password".into(),
                    secret: true,
                },
                TemplateFieldInput {
                    name: "端口".into(),
                    field_type: "number".into(),
                    secret: false,
                },
            ],
        };
        let t = create_template_db(&state.db, input.clone()).expect("创建模板失败");
        assert_eq!(t.category_name, "其它");
        assert_eq!(t.fields.len(), 3);
        assert_eq!(t.fields[0].name, "主机");
        assert_eq!(t.fields[2].field_type, "number");

        let got = load_template(&state.db, &t.id).expect("获取模板失败");
        assert_eq!(got.fields[2].name, "端口", "字段顺序应按 sort_order 保留");

        // 更新：重建字段
        let upd_input = TemplateInput {
            category_id: "custom".into(),
            name: "SSH服务器2".into(),
            icon: None,
            note: None,
            fields: vec![TemplateFieldInput {
                name: "主机".into(),
                field_type: "text".into(),
                secret: false,
            }],
        };
        let updated = update_template_db(&state.db, t.id.clone(), upd_input).expect("更新模板失败");
        assert_eq!(updated.name, "SSH服务器2");
        assert_eq!(updated.fields.len(), 1);

        // 非法类型 / 空名 / 重名校验
        let bad_type = create_template_db(
            &state.db,
            TemplateInput {
                category_id: "custom".into(),
                name: "坏模板".into(),
            icon: None,
            note: None,
                fields: vec![TemplateFieldInput {
                    name: "x".into(),
                    field_type: "hack".into(),
                    secret: false,
                }],
            },
        );
        assert!(bad_type.is_err(), "非法字段类型应报错");

        let empty_name = create_template_db(
            &state.db,
            TemplateInput {
                category_id: "custom".into(),
                name: "".into(),
            icon: None,
            note: None,
                fields: vec![TemplateFieldInput {
                    name: "x".into(),
                    field_type: "text".into(),
                    secret: false,
                }],
            },
        );
        assert!(empty_name.is_err(), "空模板名应报错");

        let dup_field = create_template_db(
            &state.db,
            TemplateInput {
                category_id: "custom".into(),
                name: "重名模板".into(),
            icon: None,
            note: None,
                fields: vec![
                    TemplateFieldInput {
                        name: "a".into(),
                        field_type: "text".into(),
                        secret: false,
                    },
                    TemplateFieldInput {
                        name: "a".into(),
                        field_type: "text".into(),
                        secret: false,
                    },
                ],
            },
        );
        assert!(dup_field.is_err(), "重名字段应报错");

        let list = list_templates_db(&state.db).expect("列表失败");
        assert!(list.iter().any(|x| x.id == t.id), "列表应包含新模板");

        let conn = state.db.conn.lock().unwrap();
        delete_template_db(&conn, t.id).expect("删除模板失败");
    }

    #[test]
    fn snapshot_roundtrip_preserves_field_type() {
        let state = unlocked_state();
        // 与 create_entry 相同的加密路径：fields 含 field_type 序列化后加密入库
        let fields_json = serde_json::to_vec(&vec![
            Field {
                name: "URL".into(),
                value: "https://example.com".into(),
                secret: false,
                field_type: "url".into(),
            },
            Field {
                name: "用户名".into(),
                value: "alice".into(),
                secret: false,
                field_type: "text".into(),
            },
            Field {
                name: "密码".into(),
                value: "s3cret".into(),
                secret: true,
                field_type: "password".into(),
            },
        ])
        .unwrap();
        let id = "snapshot-entry";
        insert_entry_raw(&state, id, &fields_json);

        let key = state.get_key().unwrap();
        let out = decode_entry(&state.db, &key, id).expect("解码条目失败");
        assert_eq!(out.fields.len(), 3);
        let url_field = out.fields.iter().find(|f| f.name == "URL").unwrap();
        assert_eq!(url_field.field_type, "url", "字段类型快照应随解密返回");
        let pw = out.fields.iter().find(|f| f.name == "密码").unwrap();
        assert_eq!(pw.field_type, "password");
        assert_eq!(pw.value, "s3cret");
    }

    #[test]
    fn old_field_payload_defaults_to_text() {
        // 旧版本无 field_type 的加密负载，解密后应默认 text
        let state = unlocked_state();
        let old_fields: Vec<serde_json::Value> = vec![
            serde_json::json!({"name": "用户名", "value": "olduser", "secret": false}),
            serde_json::json!({"name": "密码", "value": "oldpass", "secret": true}),
        ];
        let data_json = serde_json::to_vec(&old_fields).unwrap();
        let id = "legacy-entry";
        insert_entry_raw(&state, id, &data_json);

        let key = state.get_key().unwrap();
        let out = decode_entry(&state.db, &key, id).expect("读取旧条目失败");
        for f in &out.fields {
            assert_eq!(f.field_type, "text", "旧数据无 field_type 应默认 text");
        }
    }
}
