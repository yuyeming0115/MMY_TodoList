use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use uuid::Uuid;
use image::GenericImageView;
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
                font_family TEXT DEFAULT '',
                clipboard_view_mode TEXT DEFAULT 'normal'
            )",
            [],
        )?;

        // 添加 window_width, window_height 列（如果不存在）
        conn.execute(
            "ALTER TABLE settings ADD COLUMN window_width INTEGER",
            [],
        ).ok();
        conn.execute(
            "ALTER TABLE settings ADD COLUMN window_height INTEGER",
            [],
        ).ok();
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
        conn.execute(
            "ALTER TABLE settings ADD COLUMN clipboard_view_mode TEXT DEFAULT 'normal'",
            [],
        ).ok();
        conn.execute(
            "ALTER TABLE settings ADD COLUMN clipboard_stack_gap INTEGER DEFAULT 64",
            [],
        ).ok();
        conn.execute(
            "ALTER TABLE settings ADD COLUMN task_view_mode TEXT DEFAULT 'normal'",
            [],
        ).ok();

        // 添加 global_shortcut 列
        conn.execute(
            "ALTER TABLE settings ADD COLUMN global_shortcut TEXT",
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
                image_path TEXT,
                thumbnail_base64 TEXT,
                priority INTEGER DEFAULT 1,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER,
                expires_at INTEGER,
                locked INTEGER DEFAULT 0
            )",
            [],
        )?;

        // 迁移：为已有数据库添加新列
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN image_path TEXT", []).ok();
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN thumbnail_base64 TEXT", []).ok();
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN expires_at INTEGER", []).ok();
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN locked INTEGER DEFAULT 0", []).ok();

        // 初始化默认剪贴板分类（文本、图像、收藏）
        let cb_count: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_categories", [], |r| r.get(0))?;
        if cb_count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            conn.execute(
                "INSERT INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES ('builtin_text', '文本', '#4A90D9', 0, ?1)",
                [now],
            )?;
            conn.execute(
                "INSERT INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES ('builtin_image', '图像', '#28C840', 1, ?1)",
                [now],
            )?;
            conn.execute(
                "INSERT INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES ('builtin_favorite', '收藏', '#F39C12', 2, ?1)",
                [now],
            )?;
        } else {
            // 迁移：为已有数据库补充缺失的内置分类
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let now_str = now.to_string();

            let builtin_categories = [
                ("builtin_text", "文本", "#4A90D9", 0),
                ("builtin_image", "图像", "#28C840", 1),
                ("builtin_favorite", "收藏", "#F39C12", 2),
            ];

            for (id, name, color, sort_order) in &builtin_categories {
                let exists: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM clipboard_categories WHERE id = ?1",
                    [id],
                    |r| r.get(0),
                )?;
                if exists == 0 {
                    let sort_order_str = sort_order.to_string();
                    conn.execute(
                        "INSERT OR IGNORE INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                        [*id, *name, *color, sort_order_str.as_str(), now_str.as_str()],
                    ).ok();

                    // 将同名的旧分类项目迁移到内置分类
                    let old_ids: Vec<String> = conn.prepare(
                        "SELECT id FROM clipboard_categories WHERE name = ?1 AND id != ?2"
                    ).ok().map(|mut stmt| {
                        stmt.query_map([*name], |r| r.get(0)).ok()
                            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<String>>())
                            .unwrap_or_default()
                    }).unwrap_or_default();

                    for old_id in &old_ids {
                        conn.execute(
                            "UPDATE clipboard_items SET category_id = ?1 WHERE category_id = ?2",
                            [*id, old_id.as_str()],
                        ).ok();
                        conn.execute(
                            "DELETE FROM clipboard_categories WHERE id = ?1",
                            [old_id.as_str()],
                        ).ok();
                    }
                }
            }
        }

        // 启动时清理过期剪贴板项目
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        conn.execute(
            "DELETE FROM clipboard_items WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            [&now.to_string()],
        ).ok();

        // 清理孤立图片文件（可选：扫描数据库，删除不在数据库中的图片文件）
        // 为性能考虑，此操作暂不自动执行

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

    /// 导入分类（使用原有 ID，用于备份恢复）
    pub fn import_category(&self, category: &Category) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&category.id, &category.name, &category.color, &category.sort_order.to_string(), &category.created_at.to_string()],
        )?;
        Ok(())
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

    /// 导入任务（保留原有字段，用于备份恢复）
    pub fn import_task(&self, task: &Task) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tasks (id, category_id, title, description, start_date, due_date,
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
                &task.sort_order,
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
            "SELECT theme_mode, language, hide_completed_tasks, launch_at_startup, window_width, window_height, window_x, window_y, font_size, font_family, clipboard_view_mode, clipboard_stack_gap, task_view_mode, global_shortcut FROM settings WHERE id = 1",
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
                clipboard_view_mode: row.get(10).unwrap_or_else(|_| "normal".to_string()),
                clipboard_stack_gap: row.get::<_, i32>(11).unwrap_or(64),
                task_view_mode: row.get(12).unwrap_or_else(|_| "normal".to_string()),
                global_shortcut: row.get::<_, Option<String>>(13)?,
            }),
        )
    }

    pub fn update_settings(&self, settings: &AppSettings) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE settings SET theme_mode = ?1, language = ?2, hide_completed_tasks = ?3, launch_at_startup = ?4, window_width = ?5, window_height = ?6, window_x = ?7, window_y = ?8, font_size = ?9, font_family = ?10, clipboard_view_mode = ?11, clipboard_stack_gap = ?12, task_view_mode = ?13, global_shortcut = ?14 WHERE id = 1",
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
                &settings.clipboard_view_mode,
                &settings.clipboard_stack_gap,
                &settings.task_view_mode,
                &settings.global_shortcut,
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

    /// 导入剪贴板分类（使用原有 ID，用于备份恢复）
    pub fn import_clipboard_category(&self, category: &ClipboardCategory) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO clipboard_categories (id, name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            [&category.id, &category.name, &category.color, &category.sort_order.to_string(), &category.created_at.to_string()],
        )?;
        Ok(())
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let mut stmt = conn.prepare(
            "SELECT id, category_id, title, content, image_base64, image_path, thumbnail_base64, priority, sort_order, created_at, expires_at, locked FROM clipboard_items WHERE expires_at IS NULL OR expires_at > ?1 ORDER BY sort_order"
        )?;

        let items = stmt.query_map([now], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                category_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                image_base64: row.get(4)?,
                image_path: row.get(5)?,
                thumbnail_base64: row.get(6)?,
                priority: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
                expires_at: row.get(10)?,
                locked: row.get::<_, Option<i32>>(11)?.map(|v| v != 0),
            })
        })?.collect::<SqliteResult<Vec<ClipboardItem>>>();

        items
    }

    pub fn add_clipboard_item(&self, item: &ClipboardItem) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO clipboard_items (id, category_id, title, content, image_base64, image_path, thumbnail_base64, priority, sort_order, created_at, expires_at, locked) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                &item.id,
                &item.category_id,
                &item.title,
                &item.content,
                &item.image_base64,
                &item.image_path,
                &item.thumbnail_base64,
                &item.priority,
                &item.sort_order,
                &item.created_at,
                &item.expires_at,
                item.locked.map(|v| if v { 1 } else { 0 }),
            ],
        )?;
        Ok(())
    }

    pub fn update_clipboard_item(&self, item: &ClipboardItem) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_items SET category_id = ?1, title = ?2, content = ?3, image_base64 = ?4, image_path = ?5, thumbnail_base64 = ?6, priority = ?7, sort_order = ?8, expires_at = ?9, locked = ?10 WHERE id = ?11",
            rusqlite::params![
                &item.category_id,
                &item.title,
                &item.content,
                &item.image_base64,
                &item.image_path,
                &item.thumbnail_base64,
                &item.priority,
                &item.sort_order,
                &item.expires_at,
                item.locked.map(|v| if v { 1 } else { 0 }),
                &item.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_clipboard_item(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        // Get image_path before deleting
        let image_path: Option<String> = conn.query_row(
            "SELECT image_path FROM clipboard_items WHERE id = ?1",
            [id],
            |row| row.get(0),
        ).ok();
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", [id])?;
        // Delete image file if it exists
        if let Some(path) = image_path {
            let p = std::path::PathBuf::from(&path);
            if p.exists() {
                std::fs::remove_file(p).ok();
            }
        }
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

        let title = if content.chars().count() > 30 {
            format!("{}...", content.chars().take(30).collect::<String>())
        } else {
            content.to_string()
        };

        // 使用内置"文本"分类 ID，30 天过期
        let category_id = "builtin_text";
        let expires_at = now + (30 * 24 * 60 * 60 * 1000);

        // 获取最小 sort_order，新内容放在最前面
        let min_order: i32 = conn.query_row(
            "SELECT COALESCE(MIN(sort_order), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;

        // 使用 INSERT OR IGNORE 防止竞态重复
        let affected = conn.execute(
            "INSERT OR IGNORE INTO clipboard_items (id, category_id, title, content, image_base64, image_path, thumbnail_base64, priority, sort_order, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, 2, ?5, ?6, ?7)",
            rusqlite::params![&id, category_id, &title, content, min_order - 1, &now, expires_at],
        )?;
        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    /// 自动保存图片剪贴板内容（用于后台监控）
    pub fn add_auto_clipboard_image(&self, image_base64: &str) -> SqliteResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 7 天过期
        let expires_at = now + (7 * 24 * 60 * 60 * 1000);

        let (image_path, thumbnail_base64) = self.save_clipboard_image(&id, image_base64)
            .map_err(|e| {
                eprintln!("[剪贴板] 保存图片失败: {:?}", e);
                e
            })?;

        // 使用内置"图像"分类 ID
        let category_id = "builtin_image";

        // 获取最小 sort_order
        let conn = self.conn.lock().unwrap();
        let min_order: i32 = conn.query_row(
            "SELECT COALESCE(MIN(sort_order), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;

        // 使用 INSERT OR IGNORE 防止竞态重复
        let affected = conn.execute(
            "INSERT OR IGNORE INTO clipboard_items (id, category_id, title, content, image_base64, image_path, thumbnail_base64, priority, sort_order, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 2, ?8, ?9, ?10)",
            rusqlite::params![&id, category_id, "剪贴板图片", "", "", image_path, thumbnail_base64, min_order - 1, &now, expires_at],
        ).map_err(|e| {
            eprintln!("[剪贴板] 插入数据库失败: {:?}", e);
            e
        })?;
        if affected == 0 {
            eprintln!("[剪贴板] INSERT OR IGNORE 未插入任何行（可能主键冲突）");
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    /// 保存剪贴板图片到文件系统，返回 (文件路径, 缩略图 base64)
    pub fn save_clipboard_image(&self, id: &str, image_base64: &str) -> SqliteResult<(String, String)> {
        // 去掉 data:image/...;base64, 前缀
        let base64_data = if let Some(idx) = image_base64.find(",") {
            &image_base64[idx + 1..]
        } else {
            image_base64
        };

        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
            .map_err(|e| {
                eprintln!("[剪贴板] base64 解码失败: {:?}", e);
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
            })?;

        // 获取应用数据目录
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let img_dir = app_dir.join("clipboard_images");
        std::fs::create_dir_all(&img_dir).map_err(|e| {
            eprintln!("[剪贴板] 创建图片目录失败: {:?}, 路径: {:?}", e, img_dir);
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        let file_path = img_dir.join(format!("{}.png", id));
        let file_path_str = file_path.to_str().unwrap_or("").to_string();

        // 保存原图
        std::fs::write(&file_path, &decoded)
            .map_err(|e| {
                eprintln!("[剪贴板] 写入图片文件失败: {:?}, 路径: {:?}", e, file_path);
                rusqlite::Error::ToSqlConversionFailure(Box::new(e))
            })?;

        // 生成缩略图（最大宽度 300px）
        let thumbnail_base64 = self.generate_thumbnail(&decoded)
            .map_err(|e| {
                eprintln!("[剪贴板] 生成缩略图失败: {:?}", e);
                e
            })?;

        Ok((file_path_str, thumbnail_base64))
    }

    /// 生成缩略图 base64（最大宽度 300px）
    fn generate_thumbnail(&self, image_data: &[u8]) -> SqliteResult<String> {
        use base64::Engine;
        use image::{ImageFormat, ImageReader};

        let img = ImageReader::with_format(std::io::Cursor::new(image_data), ImageFormat::Png)
            .decode()
            .map_err(|e| {
                eprintln!("[剪贴板] 缩略图解码失败: {:?}", e);
                rusqlite::Error::ToSqlConversionFailure(Box::new(e))
            })?;

        let (width, height) = img.dimensions();
        let max_width: u32 = 300;
        let thumbnail = if width > max_width {
            let new_height = (max_width as f32 / width as f32 * height as f32) as u32;
            img.resize(max_width, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        let mut thumbnail_buf = Vec::new();
        thumbnail.write_to(
            &mut std::io::Cursor::new(&mut thumbnail_buf),
            image::ImageFormat::Png,
        ).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        let thumbnail_b64 = base64::engine::general_purpose::STANDARD.encode(&thumbnail_buf);
        Ok(format!("data:image/png;base64,{}", thumbnail_b64))
    }

    /// 根据文件路径读取原图字节（用于写入系统剪贴板）
    pub fn read_clipboard_image_file(&self, path: &str) -> SqliteResult<Vec<u8>> {
        let p = std::path::PathBuf::from(path);
        if !p.exists() {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        std::fs::read(&p).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }
}