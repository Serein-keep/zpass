use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// 数据库连接（全局单例）
pub struct DbState {
    pub conn: Mutex<Connection>,
}

impl DbState {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let conn = Connection::open(&path).map_err(|e| format!("打开数据库失败: {}", e))?;
        let db = DbState {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS master_key (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                hash TEXT NOT NULL,
                salt TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                title TEXT NOT NULL,
                icon TEXT,
                note TEXT,
                encrypted_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT
            );

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                color TEXT
            );

            CREATE TABLE IF NOT EXISTS entry_tags (
                entry_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag_id)
            );

            CREATE TABLE IF NOT EXISTS custom_fields (
                id TEXT PRIMARY KEY,
                entry_id TEXT NOT NULL,
                name TEXT NOT NULL,
                value_encrypted TEXT NOT NULL,
                sort_order INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS otp_entries (
                id TEXT PRIMARY KEY,
                issuer TEXT NOT NULL,
                account TEXT NOT NULL,
                secret TEXT NOT NULL,
                interval INTEGER NOT NULL DEFAULT 30,
                digits INTEGER NOT NULL DEFAULT 6,
                algorithm TEXT NOT NULL DEFAULT 'SHA1',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS template_categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                icon TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                name TEXT NOT NULL UNIQUE,
                icon TEXT,
                color TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                category_id TEXT NOT NULL,
                name TEXT NOT NULL,
                icon TEXT,
                note TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS template_fields (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                name TEXT NOT NULL,
                field_type TEXT NOT NULL DEFAULT 'text',
                secret INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
        "#,
        )
        .map_err(|e| format!("初始化数据库失败: {}", e))?;

        // 迁移：为 entries 表添加 OTP 关联列（旧版本数据无此列）
        let has_otp_id: bool = conn
            .prepare("PRAGMA table_info(entries)")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get::<usize, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .any(|name| name == "otp_id");
        if !has_otp_id {
            conn.execute_batch(
                "ALTER TABLE entries ADD COLUMN otp_id TEXT;
                 ALTER TABLE entries ADD COLUMN otp_mode TEXT;",
            )
            .map_err(|e| format!("迁移 entries 表失败: {}", e))?;
        }

        // 迁移：移除 entries.template_id 列（模板关联已废弃，密码条目不再关联模板）
        let has_template_id: bool = conn
            .prepare("PRAGMA table_info(entries)")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get::<usize, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .any(|name| name == "template_id");
        if has_template_id {
            conn.execute_batch("ALTER TABLE entries DROP COLUMN template_id;")
                .map_err(|e| format!("迁移 entries.template_id 失败: {}", e))?;
        }

        // 迁移：templates 表补充 icon/note 列、template_categories 表补充 icon 列（旧版本无此列）
        let table_cols = |table: &str| -> Result<Vec<String>, String> {
            let cols = conn
                .prepare(&format!("PRAGMA table_info({})", table))
                .map_err(|e| e.to_string())?
                .query_map([], |r| r.get::<usize, String>(1))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(cols)
        };
        let tmpl_cols = table_cols("templates")?;
        if !tmpl_cols.contains(&"icon".to_string()) {
            conn.execute_batch("ALTER TABLE templates ADD COLUMN icon TEXT;")
                .map_err(|e| format!("迁移 templates.icon 失败: {}", e))?;
        }
        if !tmpl_cols.contains(&"note".to_string()) {
            conn.execute_batch("ALTER TABLE templates ADD COLUMN note TEXT;")
                .map_err(|e| format!("迁移 templates.note 失败: {}", e))?;
        }
        let cat_cols = table_cols("template_categories")?;
        if !cat_cols.contains(&"icon".to_string()) {
            conn.execute_batch("ALTER TABLE template_categories ADD COLUMN icon TEXT;")
                .map_err(|e| format!("迁移 template_categories.icon 失败: {}", e))?;
        }

        self.seed_builtin_templates(&conn)?;

        // 默认设置
        let defaults: Vec<(&str, &str)> = vec![
            ("lock_timeout", "30"),
            ("theme", "light"),
            ("storage_path", ""),
        ];
        for (k, v) in defaults {
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )
            .map_err(|e| format!("初始化设置失败: {}", e))?;
        }
        Ok(())
    }

    /// 内置模板种子：固定 ID 保证 INSERT OR IGNORE 幂等
    fn seed_builtin_templates(&self, conn: &rusqlite::Connection) -> Result<(), String> {
        let now = crate::state::now_secs().to_string();

        // 内置类别（与 entries.category 取值兼容）
        let categories: Vec<(&str, &str, &str, &str)> = vec![
            ("login", "登录", "globe", "#18a058"),
            ("database", "数据库", "server", "#2080f0"),
            ("email", "邮箱", "mail", "#f0a020"),
            ("custom", "其它", "folder-open", "#7339d8"),
        ];
        for (id, name, icon, color) in categories {
            conn.execute(
                "INSERT OR IGNORE INTO categories (id, parent_id, name, icon, color, sort_order, created_at) VALUES (?1, NULL, ?2, ?3, ?4, 0, ?5)",
                params![id, name, icon, color, now],
            )
            .map_err(|e| format!("初始化类别失败: {}", e))?;
        }

        // 迁移旧模板类别：template_categories 的行复制到 categories（保留 id），模板分类即主类别
        conn.execute(
            "INSERT OR IGNORE INTO categories (id, parent_id, name, icon, color, sort_order, created_at) SELECT id, NULL, name, icon, NULL, 100, created_at FROM template_categories WHERE id NOT IN ('cat-builtin')",
            [],
        )
        .map_err(|e| format!("迁移模板类别失败: {}", e))?;

        // 内置模板（category_id 指向内置类别）
        let templates: Vec<(&str, &str, &str, &str, &str)> = vec![
            ("tmpl-login", "login", "登录", "globe", "网站账号登录信息"),
            ("tmpl-database", "database", "数据库", "server", "数据库连接信息"),
            ("tmpl-email", "email", "邮箱", "mail", "邮箱账号与邮件服务器"),
            ("tmpl-custom", "custom", "其它", "document-text", ""),
        ];
        for (id, category_id, name, icon, note) in templates {
            conn.execute(
                "INSERT OR IGNORE INTO templates (id, category_id, name, icon, note, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, category_id, name, icon, note, now],
            )
            .map_err(|e| format!("初始化模板失败: {}", e))?;
        }
        // 旧库中内置模板可能仍指向 cat-builtin，统一修正
        conn.execute(
            "UPDATE templates SET category_id = 'login' WHERE id = 'tmpl-login'",
            [],
        )
        .map_err(|e| format!("修正内置模板类别失败: {}", e))?;
        conn.execute(
            "UPDATE templates SET category_id = 'database' WHERE id = 'tmpl-database'",
            [],
        )
        .map_err(|e| format!("修正内置模板类别失败: {}", e))?;
        conn.execute(
            "UPDATE templates SET category_id = 'email' WHERE id = 'tmpl-email'",
            [],
        )
        .map_err(|e| format!("修正内置模板类别失败: {}", e))?;
        conn.execute(
            "UPDATE templates SET category_id = 'custom' WHERE id = 'tmpl-custom'",
            [],
        )
        .map_err(|e| format!("修正内置模板类别失败: {}", e))?;

        // (template_id, 字段id后缀, name, field_type, secret, sort_order)
        let fields: Vec<(&str, &str, &str, &str, i64, i64)> = vec![
            ("tmpl-login", "1", "URL", "url", 0, 0),
            ("tmpl-login", "2", "用户名", "text", 0, 1),
            ("tmpl-login", "3", "密码", "password", 1, 2),
            ("tmpl-database", "1", "主机", "text", 0, 0),
            ("tmpl-database", "2", "端口", "text", 0, 1),
            ("tmpl-database", "3", "数据库名", "text", 0, 2),
            ("tmpl-database", "4", "用户名", "text", 0, 3),
            ("tmpl-database", "5", "密码", "password", 1, 4),
            ("tmpl-email", "1", "邮箱地址", "email", 0, 0),
            ("tmpl-email", "2", "密码", "password", 1, 1),
            ("tmpl-email", "3", "SMTP服务器", "text", 0, 2),
            ("tmpl-email", "4", "端口", "text", 0, 3),
            ("tmpl-custom", "1", "用户", "text", 0, 0),
            ("tmpl-custom", "2", "密码", "password", 1, 1),
        ];
        for (template_id, suffix, name, field_type, secret, sort_order) in fields {
            let id = format!("tf-{}-{}", template_id, suffix);
            conn.execute(
                "INSERT OR IGNORE INTO template_fields (id, template_id, name, field_type, secret, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, template_id, name, field_type, secret, sort_order],
            )
            .map_err(|e| format!("初始化模板字段失败: {}", e))?;
        }
        Ok(())
    }
}

/// 获取默认数据库路径：用户数据目录/zpass.db
pub fn default_db_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("zpass");
    std::fs::create_dir_all(&path).ok();
    path.push("zpass.db");
    path
}
