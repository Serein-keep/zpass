pub mod auth;
pub mod categories;
pub mod entries;
pub mod export;
pub mod otp;
pub mod settings;
pub mod stats;
pub mod tags;
pub mod templates;

pub use auth::*;
pub use categories::*;
pub use entries::*;
pub use export::*;
pub use otp::*;
pub use settings::*;
pub use stats::*;
pub use tags::*;
pub use templates::*;

use crate::crypto::CryptoKey;
use crate::state::AppState;
use rusqlite::params;

pub type Result<T> = std::result::Result<T, String>;

/// 内部：获取密钥或报错
pub fn require_key(state: &AppState) -> Result<CryptoKey> {
    state.get_key().ok_or_else(|| "应用已锁定，请先解锁".to_string())
}

pub fn ensure_db_unlocked(state: &AppState) -> Result<()> {
    if state.get_key().is_none() {
        return Err("应用已锁定".into());
    }
    Ok(())
}

pub fn upsert_tag(conn: &rusqlite::Connection, name: &str) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::CryptoKey;
    use crate::db::DbState;
    use crate::models::*;

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
    fn insert_entry_raw(state: &AppState, id: &str, fields_json: &[u8]) {
        let key = state.get_key().unwrap();
        let enc = crate::crypto::encrypt(&key.key, fields_json).unwrap();
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

        let child = super::categories::create_category_db(
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

        let dup = super::categories::create_category_db(
            &conn,
            CategoryInput {
                name: "公司邮箱".into(),
                parent_id: None,
                icon: None,
                color: None,
            },
        );
        assert!(dup.is_err(), "重名类别应报错");

        let updated = super::categories::update_category_db(
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

        let self_parent = super::categories::update_category_db(
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

        let del_child = super::categories::delete_category_db(&conn, "email".into());
        assert!(del_child.is_err(), "有子类别的类别删除应被阻止");

        let del_with_tmpl = super::categories::delete_category_db(&conn, "login".into());
        assert!(del_with_tmpl.is_err(), "有模板的类别删除应被阻止");

        let del_with_entry = super::categories::delete_category_db(&conn, "custom".into());
        assert!(del_with_entry.is_err(), "有模板的类别删除应被阻止");

        super::categories::delete_category_db(&conn, child.id).expect("删除子类别失败");
        let del_parent = super::categories::delete_category_db(&conn, "email".into());
        assert!(del_parent.is_err(), "有模板的 email 类别删除应被阻止");

        let tmp = super::categories::create_category_db(
            &conn,
            CategoryInput {
                name: "临时类别".into(),
                parent_id: None,
                icon: None,
                color: None,
            },
        )
        .expect("创建临时类别失败");
        super::categories::delete_category_db(&conn, tmp.id).expect("删除空类别失败");
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
        let t = super::templates::create_template_db(&state.db, input.clone()).expect("创建模板失败");
        assert_eq!(t.category_name, "其它");
        assert_eq!(t.fields.len(), 3);
        assert_eq!(t.fields[0].name, "主机");
        assert_eq!(t.fields[2].field_type, "number");

        let got = super::templates::load_template(&state.db, &t.id).expect("获取模板失败");
        assert_eq!(got.fields[2].name, "端口", "字段顺序应按 sort_order 保留");

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
        let updated = super::templates::update_template_db(&state.db, t.id.clone(), upd_input).expect("更新模板失败");
        assert_eq!(updated.name, "SSH服务器2");
        assert_eq!(updated.fields.len(), 1);

        let bad_type = super::templates::create_template_db(
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

        let empty_name = super::templates::create_template_db(
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

        let dup_field = super::templates::create_template_db(
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

        let list = super::templates::list_templates_db(&state.db).expect("列表失败");
        assert!(list.iter().any(|x| x.id == t.id), "列表应包含新模板");

        let conn = state.db.conn.lock().unwrap();
        super::templates::delete_template_db(&conn, t.id).expect("删除模板失败");
    }

    #[test]
    fn snapshot_roundtrip_preserves_field_type() {
        let state = unlocked_state();
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
        let out = super::entries::decode_entry(&state.db, &key, id).expect("解码条目失败");
        assert_eq!(out.fields.len(), 3);
        let url_field = out.fields.iter().find(|f| f.name == "URL").unwrap();
        assert_eq!(url_field.field_type, "url", "字段类型快照应随解密返回");
        let pw = out.fields.iter().find(|f| f.name == "密码").unwrap();
        assert_eq!(pw.field_type, "password");
        assert_eq!(pw.value, "s3cret");
    }

    #[test]
    fn old_field_payload_defaults_to_text() {
        let state = unlocked_state();
        let old_fields: Vec<serde_json::Value> = vec![
            serde_json::json!({"name": "用户名", "value": "olduser", "secret": false}),
            serde_json::json!({"name": "密码", "value": "oldpass", "secret": true}),
        ];
        let data_json = serde_json::to_vec(&old_fields).unwrap();
        let id = "legacy-entry";
        insert_entry_raw(&state, id, &data_json);

        let key = state.get_key().unwrap();
        let out = super::entries::decode_entry(&state.db, &key, id).expect("读取旧条目失败");
        for f in &out.fields {
            assert_eq!(f.field_type, "text", "旧数据无 field_type 应默认 text");
        }
    }
}
