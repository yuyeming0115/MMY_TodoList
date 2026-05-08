use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use uuid::Uuid;
use crate::models::{Category, Task, AppSettings, ClipboardCategory, ClipboardItem};

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn init(app_handle: &tauri::AppHandle) -> SqliteResult<Self> {
        let app_dir = app_handle.path().app_data_dir().expect("无法获取应用目录");
        std::fs::create_dir_all(&app_dir).expect("无法创建应用目录");
        let db_path = PathBuf::from(&app_dir).join("mmy_todo.db");

        let conn = Connection::open(&db_path)?;

        // 创建表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                category_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                start_date INTEGER,
                due_date INTEGER,
                priority INTEGER DEFAULT 1,
                status TEXT DEFAULT 'todo',
                sort_order INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                thumbnail_base64 TEXT,
                created_at INTEGER,
                updated_at INTEGER
            )",
            [],
        )?;

        // 添加 is_pinned 列（如果不存在）
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN is_pinned INTEGER DEFAULT 0",
            [],
        ).ok(); // 忽略错误（列已存在）

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                theme_mode TEXT DEFAULT 'system',
                language TEXT DEFAULT 'zh',
                hide_completed_tasks INTEGER DEFAULT 0,
                launch_at_startup INTEGER DEFAULT 0,
                window_width INTEGER,
                window_height INTEGER,
                window_x INTEGER,
                window_y INTEGER,
                font_size INTEGER DEFAULT 14,
                font_family TEXT DEFAULT ''
            )",
            [],
        )?;

        // 添加 window_x, window_y 列（如果不存在）
        conn.execute(
            "ALTER TABLE settings ADD COLUMN window_x INTEGER",
            [],
        ).ok(); // 忽略错误（列已存在）
        conn.execute(
            "ALTER TABLE settings ADD COLUMN window_y INTEGER",
            [],
        ).ok(); // 忽略错误（列已存在）
        conn.execute(
            "ALTER TABLE settings ADD COLUMN font_size INTEGER DEFAULT 14",
            [],
        ).ok();
        conn.execute(
            "ALTER TABLE settings ADD COLUMN font_family TEXT DEFAULT ''",
            [],
        ).ok();

        // 初始化默认设置
        conn.execute(
            "INSERT OR IGNORE INTO settings (id) VALUES (1)",
            [],
        )?;

        // 创建剪贴板分类表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )?;

        // 创建剪贴板项目表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                category_id TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                image_base64 TEXT,
                priority INTEGER DEFAULT 1,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER
            )",
            [],
        )?;

        // 初始化默认剪贴板分类
        let cb_count: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_categories", [], |r| r.get(0))?;
        if cb_count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            conn.execute(
                "INSERT INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES ('cb-default', '默认', '#4A90D9', 0, ?1)",
                [now],
            )?;
        }

        // 初始化默认分类（仅当数据库中没有任何分类时）
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
        if count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            conn.execute(
                "INSERT INTO categories (id, name, color, sort_order, created_at) VALUES ('default-home', '家务', '#28C840', 0, ?1)",
                [now],
            )?;

            conn.execute(
                "INSERT INTO categories (id, name, color, sort_order, created_at) VALUES ('default-work', '工作', '#4A90D9', 1, ?1)",
                [now],
            )?;
        }

        Ok(Self { conn: Mutex::new(conn) })
    }

    // 分类操作
    pub fn get_categories(&self) -> SqliteResult<Vec<Category>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, sort_order, created_at FROM categories ORDER BY sort_order"
        )?;

        let categories = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?.collect::<SqliteResult<Vec<Category>>>();

        categories
    }

    pub fn add_category(&self, name: String, color: String) -> SqliteResult<Category> {
        let category = Category::new(name, color);
        let conn = self.conn.lock().unwrap();

        // 获取最大 sort_order
        let max_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM categories",
            [],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&category.id, &category.name, &category.color, &max_order.to_string(), &category.created_at.to_string()],
        )?;

        Ok(category)
    }

    pub fn update_category(&self, category: &Category) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE categories SET name = ?1, color = ?2, sort_order = ?3 WHERE id = ?4",
            [&category.name, &category.color, &category.sort_order.to_string(), &category.id],
        )?;
        Ok(())
    }

    pub fn delete_category(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
        conn.execute("DELETE FROM tasks WHERE category_id = ?1", [id])?;
        Ok(())
    }

    pub fn ensure_default_category(&self) -> Result<String, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
        if count > 0 {
            let id: String = conn.query_row(
                "SELECT id FROM categories ORDER BY sort_order LIMIT 1",
                [], |r| r.get(0)
            )?;
            return Ok(id);
        }
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&id, &"默认".to_string(), &"#4A90D9".to_string(), &0.to_string(), &now.to_string()],
        )?;
        Ok(id)
    }

    pub fn reorder_categories(&self, ids: &[String]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE categories SET sort_order = ?1 WHERE id = ?2",
                [&(i as i32).to_string(), id],
            )?;
        }
        Ok(())
    }

    // 任务操作
    pub fn get_tasks(&self) -> SqliteResult<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, category_id, title, description, start_date, due_date,
                    priority, status, sort_order, is_pinned, thumbnail_base64, created_at, updated_at
            FROM tasks ORDER BY is_pinned DESC, sort_order"
        )?;

        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                category_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                start_date: row.get(4)?,
                due_date: row.get(5)?,
                priority: row.get(6)?,
                status: row.get(7)?,
                sort_order: row.get(8)?,
                is_pinned: row.get::<_, i32>(9)? != 0,
                thumbnail_base64: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?.collect::<SqliteResult<Vec<Task>>>();

        tasks
    }

    pub fn add_task(&self, task: &Task) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        // 获取最大 sort_order
        let max_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM tasks",
            [],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO tasks (id, category_id, title, description, start_date, due_date,
                               priority, status, sort_order, is_pinned, thumbnail_base64, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                &task.id,
                &task.category_id,
                &task.title,
                &task.description,
                &task.start_date,
                &task.due_date,
                &task.priority,
                &task.status,
                max_order + 1,
                if task.is_pinned { 1 } else { 0 },
                &task.thumbnail_base64,
                &task.created_at,
                &task.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_task(&self, task: &Task) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET category_id = ?1, title = ?2, description = ?3,
                              start_date = ?4, due_date = ?5, priority = ?6, status = ?7,
                              sort_order = ?8, is_pinned = ?9, thumbnail_base64 = ?10, updated_at = ?11
             WHERE id = ?12",
            rusqlite::params![
                &task.category_id,
                &task.title,
                &task.description,
                &task.start_date,
                &task.due_date,
                &task.priority,
                &task.status,
                &task.sort_order,
                if task.is_pinned { 1 } else { 0 },
                &task.thumbnail_base64,
                &task.updated_at,
                &task.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_task(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn reorder_tasks(&self, ids: &[String]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                [&(i as i32).to_string(), id],
            )?;
        }
        Ok(())
    }

    pub fn reset_task_sort(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET sort_order =
                (SELECT COUNT(*) FROM tasks t2 WHERE t2.priority > tasks.priority OR
                 (t2.priority = tasks.priority AND t2.due_date < tasks.due_date))",
            [],
        )?;
        Ok(())
    }

    // 设置操作
    pub fn get_settings(&self) -> SqliteResult<AppSettings> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT theme_mode, language, hide_completed_tasks, launch_at_startup, window_width, window_height, window_x, window_y, font_size, font_family FROM settings WHERE id = 1",
            [],
            |row| Ok(AppSettings {
                theme_mode: row.get(0)?,
                language: row.get(1)?,
                hide_completed_tasks: row.get(2)?,
                launch_at_startup: row.get(3)?,
                window_width: row.get::<_, Option<i32>>(4)?,
                window_height: row.get::<_, Option<i32>>(5)?,
                window_x: row.get::<_, Option<i32>>(6)?,
                window_y: row.get::<_, Option<i32>>(7)?,
                font_size: row.get::<_, i32>(8)?,
                font_family: row.get(9)?,
            }),
        )
    }

    pub fn update_settings(&self, settings: &AppSettings) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE settings SET theme_mode = ?1, language = ?2, hide_completed_tasks = ?3, launch_at_startup = ?4, window_width = ?5, window_height = ?6, window_x = ?7, window_y = ?8, font_size = ?9, font_family = ?10 WHERE id = 1",
            rusqlite::params![
                &settings.theme_mode,
                &settings.language,
                &settings.hide_completed_tasks,
                &settings.launch_at_startup,
                &settings.window_width,
                &settings.window_height,
                &settings.window_x,
                &settings.window_y,
                &settings.font_size,
                &settings.font_family,
            ],
        )?;
        Ok(())
    }

    // 剪贴板分类操作
    pub fn get_clipboard_categories(&self) -> SqliteResult<Vec<ClipboardCategory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, sort_order, created_at FROM clipboard_categories ORDER BY sort_order"
        )?;

        let categories = stmt.query_map([], |row| {
            Ok(ClipboardCategory {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?.collect::<SqliteResult<Vec<ClipboardCategory>>>();

        categories
    }

    pub fn add_clipboard_category(&self, name: String, color: String) -> SqliteResult<ClipboardCategory> {
        let category = ClipboardCategory::new(name, color);
        let conn = self.conn.lock().unwrap();

        let max_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM clipboard_categories",
            [],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&category.id, &category.name, &category.color, &max_order.to_string(), &category.created_at.to_string()],
        )?;

        Ok(category)
    }

    pub fn update_clipboard_category(&self, category: &ClipboardCategory) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_categories SET name = ?1, color = ?2, sort_order = ?3 WHERE id = ?4",
            [&category.name, &category.color, &category.sort_order.to_string(), &category.id],
        )?;
        Ok(())
    }

    pub fn delete_clipboard_category(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_items WHERE category_id = ?1", [id])?;
        conn.execute("DELETE FROM clipboard_categories WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn reorder_clipboard_categories(&self, ids: &[String]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE clipboard_categories SET sort_order = ?1 WHERE id = ?2",
                [&(i as i32).to_string(), id],
            )?;
        }
        Ok(())
    }

    // 剪贴板项目操作
    pub fn get_clipboard_items(&self) -> SqliteResult<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, category_id, title, content, image_base64, priority, sort_order, created_at FROM clipboard_items ORDER BY sort_order"
        )?;

        let items = stmt.query_map([], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                category_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                image_base64: row.get(4)?,
                priority: row.get(5)?,
                sort_order: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?.collect::<SqliteResult<Vec<ClipboardItem>>>();

        items
    }

    pub fn add_clipboard_item(&self, item: &ClipboardItem) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_items (id, category_id, title, content, image_base64, priority, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &item.id,
                &item.category_id,
                &item.title,
                &item.content,
                &item.image_base64,
                &item.priority,
                &item.sort_order,
                &item.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_clipboard_item(&self, item: &ClipboardItem) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_items SET category_id = ?1, title = ?2, content = ?3, image_base64 = ?4, priority = ?5, sort_order = ?6 WHERE id = ?7",
            rusqlite::params![
                &item.category_id,
                &item.title,
                &item.content,
                &item.image_base64,
                &item.priority,
                &item.sort_order,
                &item.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_clipboard_item(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn reorder_clipboard_items(&self, ids: &[String]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE clipboard_items SET sort_order = ?1 WHERE id = ?2",
                [&(i as i32).to_string(), id],
            )?;
        }
        Ok(())
    }

    /// 检查文本内容是否已存在于剪贴板项目中（去重用）
    pub fn clipboard_text_exists(&self, content: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE content = ?1 AND image_base64 IS NULL",
            [content],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 自动保存文本剪贴板内容（用于后台监控）
    pub fn add_auto_clipboard_text(&self, content: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let title = if content.len() > 30 {
            format!("{}...", &content[..30])
        } else {
            content.to_string()
        };

        // 获取默认剪贴板分类 ID
        let category_id: String = conn.query_row(
            "SELECT id FROM clipboard_categories ORDER BY sort_order LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        // 获取最小 sort_order，新内容放在最前面
        let min_order: i32 = conn.query_row(
            "SELECT COALESCE(MIN(sort_order), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO clipboard_items (id, category_id, title, content, image_base64, priority, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 2, ?5, ?6)",
            rusqlite::params![&id, &category_id, &title, content, min_order - 1, &now],
        )?;
        Ok(())
    }

    /// 自动保存图片剪贴板内容（用于后台监控）
    pub fn add_auto_clipboard_image(&self, image_base64: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 获取默认剪贴板分类 ID
        let category_id: String = conn.query_row(
            "SELECT id FROM clipboard_categories ORDER BY sort_order LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        // 获取最小 sort_order
        let min_order: i32 = conn.query_row(
            "SELECT COALESCE(MIN(sort_order), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO clipboard_items (id, category_id, title, content, image_base64, priority, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 2, ?6, ?7)",
            rusqlite::params![&id, &category_id, "剪贴板图片", "", image_base64, min_order - 1, &now],
        )?;
        Ok(())
    }
}