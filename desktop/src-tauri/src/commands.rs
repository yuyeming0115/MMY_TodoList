use crate::database::Database;
use crate::models::{Category, Task, AppSettings, ExportData, ClipboardCategory, ClipboardItem};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub category_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<i64>,
    pub due_date: Option<i64>,
    pub priority: i32,
    pub status: String,
    pub sort_order: i32,
    pub thumbnail_base64: Option<String>,
}

// 分类命令
#[tauri::command]
pub fn get_categories(db: State<'_, Arc<Database>>) -> Result<Vec<Category>, String> {
    db.get_categories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_category(db: State<'_, Arc<Database>>, name: String, color: String) -> Result<Category, String> {
    db.add_category(name, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(db: State<'_, Arc<Database>>, category: Category) -> Result<(), String> {
    db.update_category(&category).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(db: State<'_, Arc<Database>>, id: String) -> Result<(), String> {
    db.delete_category(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_default_category(db: State<'_, Arc<Database>>) -> Result<String, String> {
    db.ensure_default_category().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_categories(db: State<'_, Arc<Database>>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_categories(&ids).map_err(|e| e.to_string())
}

// 任务命令
#[tauri::command]
pub fn get_tasks(db: State<'_, Arc<Database>>) -> Result<Vec<Task>, String> {
    db.get_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_task(db: State<'_, Arc<Database>>, task: NewTask) -> Result<Task, String> {
    let now = Utc::now().timestamp_millis();
    let full_task = Task {
        id: Uuid::new_v4().to_string(),
        category_id: task.category_id,
        title: task.title,
        description: task.description,
        start_date: task.start_date,
        due_date: task.due_date,
        priority: task.priority,
        status: task.status,
        sort_order: task.sort_order,
        is_pinned: false,
        thumbnail_base64: task.thumbnail_base64,
        created_at: now,
        updated_at: now,
    };
    db.add_task(&full_task).map_err(|e| e.to_string())?;
    Ok(full_task)
}

#[tauri::command]
pub fn update_task(db: State<'_, Arc<Database>>, task: Task) -> Result<(), String> {
    db.update_task(&task).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(db: State<'_, Arc<Database>>, id: String) -> Result<(), String> {
    db.delete_task(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_tasks(db: State<'_, Arc<Database>>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_tasks(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_task_sort(db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.reset_task_sort().map_err(|e| e.to_string())
}

// 设置命令
#[tauri::command]
pub fn get_settings(db: State<'_, Arc<Database>>) -> Result<AppSettings, String> {
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(db: State<'_, Arc<Database>>, settings: AppSettings) -> Result<(), String> {
    db.update_settings(&settings).map_err(|e| e.to_string())
}

// 导出导入命令
#[tauri::command]
pub fn export_data(db: State<'_, Arc<Database>>) -> Result<ExportData, String> {
    let categories = db.get_categories().map_err(|e| e.to_string())?;
    let tasks = db.get_tasks().map_err(|e| e.to_string())?;
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    Ok(ExportData {
        version: "2.0".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        source: "desktop".to_string(),
        categories,
        tasks,
        settings,
    })
}

#[tauri::command]
pub fn import_data(db: State<'_, Arc<Database>>, data: ExportData) -> Result<(), String> {
    // 清空现有数据
    {
        let conn = db.conn.lock().unwrap();
        conn.execute("DELETE FROM categories", []).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM tasks", []).map_err(|e| e.to_string())?;
    }

    // 导入分类
    for category in &data.categories {
        db.add_category(category.name.clone(), category.color.clone())
            .map_err(|e| e.to_string())?;
    }

    // 导入任务
    for task in &data.tasks {
        db.add_task(task).map_err(|e| e.to_string())?;
    }

    // 导入设置
    db.update_settings(&data.settings).map_err(|e| e.to_string())?;

    Ok(())
}

// 剪贴板分类命令
#[tauri::command]
pub fn get_clipboard_categories(db: State<'_, Arc<Database>>) -> Result<Vec<ClipboardCategory>, String> {
    db.get_clipboard_categories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_clipboard_category(db: State<'_, Arc<Database>>, name: String, color: String) -> Result<ClipboardCategory, String> {
    db.add_clipboard_category(name, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_clipboard_category(db: State<'_, Arc<Database>>, category: ClipboardCategory) -> Result<(), String> {
    db.update_clipboard_category(&category).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clipboard_category(db: State<'_, Arc<Database>>, id: String) -> Result<(), String> {
    db.delete_clipboard_category(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_clipboard_categories(db: State<'_, Arc<Database>>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_clipboard_categories(&ids).map_err(|e| e.to_string())
}

// 剪贴板项目命令
#[tauri::command]
pub fn get_clipboard_items(db: State<'_, Arc<Database>>) -> Result<Vec<ClipboardItem>, String> {
    db.get_clipboard_items().map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewClipboardItem {
    pub category_id: String,
    pub title: String,
    pub content: String,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub thumbnail_base64: Option<String>,
    pub priority: i32,
    pub sort_order: i32,
    pub expires_at: Option<i64>,
}

#[tauri::command]
pub fn add_clipboard_item(db: State<'_, Arc<Database>>, item: NewClipboardItem) -> Result<ClipboardItem, String> {
    let now = Utc::now().timestamp_millis();
    let full_item = ClipboardItem {
        id: Uuid::new_v4().to_string(),
        category_id: item.category_id,
        title: item.title,
        content: item.content,
        image_base64: item.image_base64,
        image_path: item.image_path,
        thumbnail_base64: item.thumbnail_base64,
        priority: item.priority,
        sort_order: item.sort_order,
        created_at: now,
        expires_at: item.expires_at,
    };
    db.add_clipboard_item(&full_item).map_err(|e| e.to_string())?;
    Ok(full_item)
}

#[tauri::command]
pub fn update_clipboard_item(db: State<'_, Arc<Database>>, item: ClipboardItem) -> Result<(), String> {
    db.update_clipboard_item(&item).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clipboard_item(db: State<'_, Arc<Database>>, id: String) -> Result<(), String> {
    db.delete_clipboard_item(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_clipboard_items(db: State<'_, Arc<Database>>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_clipboard_items(&ids).map_err(|e| e.to_string())
}

/// 读取剪贴板图片文件（返回 base64 data URL）
#[tauri::command]
pub fn read_clipboard_image_file(db: State<'_, Arc<Database>>, path: String) -> Result<String, String> {
    let bytes = db.read_clipboard_image_file(&path).map_err(|e| e.to_string())?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

/// 设置剪贴板项目过期时间
#[tauri::command]
pub fn set_clipboard_item_expiry(db: State<'_, Arc<Database>>, id: String, expires_at: Option<i64>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE clipboard_items SET expires_at = ?1 WHERE id = ?2",
        rusqlite::params![&expires_at, &id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// 清理所有已过期项目
#[tauri::command]
pub fn cleanup_expired_items(db: State<'_, Arc<Database>>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // 获取需要删除的项目的图片 path
    let mut stmt = conn.prepare(
        "SELECT image_path FROM clipboard_items WHERE expires_at IS NOT NULL AND expires_at <= ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([now], |row| row.get(0)).map_err(|e| e.to_string())?;
    let paths: Vec<Option<String>> = rows.filter_map(|r| r.ok()).collect();

    let deleted = conn.execute(
        "DELETE FROM clipboard_items WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        [&now.to_string()],
    ).map_err(|e| e.to_string())?;

    // 删除对应的图片文件
    for path_opt in paths {
        if let Some(path) = path_opt {
            let p = std::path::PathBuf::from(&path);
            if p.exists() {
                std::fs::remove_file(p).ok();
            }
        }
    }

    Ok(deleted as i64)
}