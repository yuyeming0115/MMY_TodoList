use crate::database::Database;
use crate::models::{Category, Task, AppSettings, ExportData, ClipboardCategory, ClipboardItem};
use crate::backup::{BackupManager, BackupSettings, BackupInfo, BackupPreview, RestoreOptions};
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
    let clipboard_categories = db.get_clipboard_categories().map_err(|e| e.to_string())?;
    let clipboard_items = db.get_clipboard_items().map_err(|e| e.to_string())?;
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    Ok(ExportData {
        version: "3.0".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        source: "desktop".to_string(),
        categories,
        tasks,
        clipboard_categories,
        clipboard_items,
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
        conn.execute("DELETE FROM clipboard_categories", []).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_items", []).map_err(|e| e.to_string())?;
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

    // 导入剪贴板分类
    for category in &data.clipboard_categories {
        db.add_clipboard_category(category.name.clone(), category.color.clone())
            .map_err(|e| e.to_string())?;
    }

    // 导入剪贴板项目
    for item in &data.clipboard_items {
        db.add_clipboard_item(item).map_err(|e| e.to_string())?;
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

/// 分页获取剪贴板项目（启动时只加载 limit 条）
#[tauri::command]
pub fn get_clipboard_items_paginated(db: State<'_, Arc<Database>>, limit: i32, offset: i32) -> Result<Vec<ClipboardItem>, String> {
    db.get_clipboard_items_paginated(limit, offset).map_err(|e| e.to_string())
}

/// 获取剪贴板项目总数
#[tauri::command]
pub fn get_clipboard_items_count(db: State<'_, Arc<Database>>) -> Result<i64, String> {
    db.get_clipboard_items_count().map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewClipboardItem {
    pub category_id: String,
    pub title: String,
    pub content: String,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    // thumbnail_base64 已移除：学习 Ditto，数据库只存路径
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
        priority: item.priority,
        sort_order: item.sort_order,
        created_at: now,
        expires_at: item.expires_at,
        locked: None,
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

/// 批量删除剪贴板项目（使用事务，一次提交）
#[tauri::command]
pub fn batch_delete_clipboard_items(db: State<'_, Arc<Database>>, ids: Vec<String>) -> Result<usize, String> {
    db.batch_delete_clipboard_items(&ids).map_err(|e| e.to_string())
}

/// 批量更新剪贴板项目分类（使用事务，一次提交）
#[tauri::command]
pub fn batch_update_clipboard_items_category(db: State<'_, Arc<Database>>, ids: Vec<String>, category_id: String) -> Result<usize, String> {
    db.batch_update_clipboard_items_category(&ids, &category_id).map_err(|e| e.to_string())
}

/// 清空所有未锁定的剪贴板项
#[tauri::command]
pub fn clear_all_unlocked_clipboard_items(db: State<'_, Arc<Database>>) -> Result<usize, String> {
    db.clear_all_unlocked_clipboard_items().map_err(|e| e.to_string())
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

    let mut stmt = conn.prepare(
        "SELECT image_path FROM clipboard_items WHERE expires_at IS NOT NULL AND expires_at <= ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([now], |row| row.get(0)).map_err(|e| e.to_string())?;
    let paths: Vec<Option<String>> = rows.filter_map(|r| r.ok()).collect();

    let deleted = conn.execute(
        "DELETE FROM clipboard_items WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        [&now.to_string()],
    ).map_err(|e| e.to_string())?;

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

/// 准备图片用于跨应用拖拽：写入 PNG + CF_HDROP 到剪贴板
/// PNG 给微信/浏览器，CF_HDROP（文件路径）给 PS 等应用
#[tauri::command]
pub fn get_image_for_drag(db: State<'_, Arc<Database>>, id: String) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let image_path: Option<String> = conn
        .query_row(
            "SELECT image_path FROM clipboard_items WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let image_path = image_path.ok_or("图片路径不存在")?;
    let bytes = std::fs::read(&image_path)
        .map_err(|e| format!("读取图片失败: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        use clipboard_win::{Clipboard, raw, formats::FileList, Setter};
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;

        let img = image::load_from_memory(&bytes).map_err(|e| format!("图片解码失败: {}", e))?;
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();

        let _clip = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))?;

        // 1. PNG 注册格式（微信、浏览器优先读取）
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            let mut png_buf = Vec::new();
            PngEncoder::new(&mut png_buf)
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| format!("PNG 编码失败: {}", e))?;
            raw::set(png_fmt.get(), &png_buf)
                .map_err(|e| format!("写入 PNG 格式失败: {}", e))?;
        }

        // 2. CF_HDROP 文件路径（PS 拖拽需要这个）
        FileList.write_clipboard(&[image_path.as_str()] as &[&str])
            .map_err(|e| format!("写入 CF_HDROP 失败: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = bytes;
    }

    Ok("ok".to_string())
}

/// 在文件管理器中打开并选中指定文件
#[tauri::command]
pub fn reveal_file_in_folder(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&path);
    if !path.exists() {
        return Err("文件不存在".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 explorer /select,path
        let path_str = path.to_string_lossy().to_string();
        std::process::Command::new("explorer")
            .args(["/select,", &path_str])
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // Mac: 使用 open -R path
        let path_str = path.to_string_lossy().to_string();
        std::process::Command::new("open")
            .args(["-R", &path_str])
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Linux: 打开包含文件夹
        let parent = path.parent().unwrap_or(&path);
        let parent_str = parent.to_string_lossy().to_string();
        std::process::Command::new("xdg-open")
            .arg(&parent_str)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }

    Ok(())
}

/// 清理图片文件已失效的剪贴板项目
#[tauri::command]
pub fn cleanup_invalid_image_items(db: State<'_, Arc<Database>>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // 查找所有有 imagePath 的项目
    let mut stmt = conn.prepare(
        "SELECT id, image_path FROM clipboard_items WHERE image_path IS NOT NULL"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let path: String = row.get(1)?;
        Ok((id, path))
    }).map_err(|e| e.to_string())?;

    let invalid_ids: Vec<String> = rows
        .filter_map(|r| r.ok())
        .filter(|(_, path)| !std::path::Path::new(path).exists())
        .map(|(id, _)| id)
        .collect();

    if invalid_ids.is_empty() {
        return Ok(0);
    }

    // 删除失效项目
    let mut deleted = 0i64;
    for id in &invalid_ids {
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
        deleted += 1;
    }

    Ok(deleted)
}

// 备份命令
#[tauri::command]
pub fn get_backup_settings(backup_mgr: State<'_, Arc<BackupManager>>) -> Result<BackupSettings, String> {
    Ok(backup_mgr.get_settings())
}

#[tauri::command]
pub fn update_backup_settings(backup_mgr: State<'_, Arc<BackupManager>>, settings: BackupSettings) -> Result<(), String> {
    backup_mgr.update_settings_internal(settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_backup_now(app: tauri::AppHandle, backup_mgr: State<'_, Arc<BackupManager>>) -> Result<String, String> {
    backup_mgr.create_backup_default(&app).ok_or("创建备份失败".to_string())
}

/// 创建指定类型的备份（quick/full）
#[tauri::command]
pub fn create_backup_with_type(
    app: tauri::AppHandle,
    backup_mgr: State<'_, Arc<BackupManager>>,
    backup_type: String,
) -> Result<String, String> {
    use crate::backup::BackupType;
    let bt = match backup_type.as_str() {
        "quick" => BackupType::Quick,
        "full" => BackupType::Full,
        _ => BackupType::Quick,
    };
    backup_mgr.create_backup(&app, bt).ok_or("创建备份失败".to_string())
}

#[tauri::command]
pub fn list_backups(backup_mgr: State<'_, Arc<BackupManager>>) -> Result<Vec<BackupInfo>, String> {
    Ok(backup_mgr.list_backups())
}

#[tauri::command]
pub fn restore_backup(backup_mgr: State<'_, Arc<BackupManager>>, filename: String) -> Result<(), String> {
    backup_mgr.restore_backup(&filename).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_backup(backup_mgr: State<'_, Arc<BackupManager>>, filename: String) -> Result<(), String> {
    backup_mgr.delete_backup(&filename).map_err(|e| e.to_string())
}

/// 预览备份内容
#[tauri::command]
pub fn preview_backup(backup_mgr: State<'_, Arc<BackupManager>>, filename: String) -> Result<BackupPreview, String> {
    backup_mgr.preview_backup(&filename).map_err(|e| e.to_string())
}

/// 选择性恢复备份（支持覆盖/合并选项）
#[tauri::command]
pub fn restore_backup_with_options(
    backup_mgr: State<'_, Arc<BackupManager>>,
    filename: String,
    options: RestoreOptions,
) -> Result<(), String> {
    backup_mgr.restore_backup_with_options(&filename, &options).map_err(|e| e.to_string())
}

/// 更新全局快捷键设置并动态注册
#[tauri::command]
pub fn update_global_shortcut(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    shortcut: Option<String>,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent};
    use tauri::Manager;

    // 更新数据库
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    let old_shortcut = settings.global_shortcut.clone();
    let mut new_settings = settings;
    new_settings.global_shortcut = shortcut.clone();
    db.update_settings(&new_settings).map_err(|e| e.to_string())?;

    // 先注销旧的（如果有）
    if let Some(old) = &old_shortcut {
        if !old.is_empty() {
            app.global_shortcut().unregister(old.as_str()).ok();
        }
    }

    // 如果有新快捷键，注册并设置处理器
    // 注意：on_shortcut 会同时注册快捷键和设置处理器
    if let Some(s) = &shortcut {
        if !s.is_empty() {
            app.global_shortcut().on_shortcut(s.as_str(), |app, _sc, event: ShortcutEvent| {
                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    if let Some(win) = app.get_webview_window("main") {
                        if win.is_minimized().unwrap_or(false) {
                            crate::show_window_at_cursor(app);
                        } else if win.is_visible().unwrap_or(false) {
                            win.hide().ok();
                        } else {
                            crate::show_window_at_cursor(app);
                        }
                    }
                }
            }).map_err(|e| format!("快捷键注册失败: {}", e))?;
        }
    }

    Ok(())
}
