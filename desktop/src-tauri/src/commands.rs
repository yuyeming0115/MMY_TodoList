use crate::database::Database;
use crate::models::{Category, Task, AppSettings, ExportData};
use chrono::Utc;
use serde::Deserialize;
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
pub fn get_categories(db: State<'_, Database>) -> Result<Vec<Category>, String> {
    db.get_categories().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_category(db: State<'_, Database>, name: String, color: String) -> Result<Category, String> {
    db.add_category(name, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(db: State<'_, Database>, category: Category) -> Result<(), String> {
    db.update_category(&category).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_category(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ensure_default_category(db: State<'_, Database>) -> Result<String, String> {
    db.ensure_default_category().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_categories(db: State<'_, Database>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_categories(&ids).map_err(|e| e.to_string())
}

// 任务命令
#[tauri::command]
pub fn get_tasks(db: State<'_, Database>) -> Result<Vec<Task>, String> {
    db.get_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_task(db: State<'_, Database>, task: NewTask) -> Result<Task, String> {
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
pub fn update_task(db: State<'_, Database>, task: Task) -> Result<(), String> {
    db.update_task(&task).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_task(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_tasks(db: State<'_, Database>, ids: Vec<String>) -> Result<(), String> {
    db.reorder_tasks(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_task_sort(db: State<'_, Database>) -> Result<(), String> {
    db.reset_task_sort().map_err(|e| e.to_string())
}

// 设置命令
#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> Result<AppSettings, String> {
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(db: State<'_, Database>, settings: AppSettings) -> Result<(), String> {
    db.update_settings(&settings).map_err(|e| e.to_string())
}

// 导出导入命令
#[tauri::command]
pub fn export_data(db: State<'_, Database>) -> Result<ExportData, String> {
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
pub fn import_data(db: State<'_, Database>, data: ExportData) -> Result<(), String> {
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