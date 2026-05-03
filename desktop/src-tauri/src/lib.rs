mod models;
mod database;
mod commands;

use database::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 初始化数据库
            let db = Database::init(&app.handle()).expect("数据库初始化失败");
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 分类命令
            commands::get_categories,
            commands::add_category,
            commands::update_category,
            commands::delete_category,
            commands::ensure_default_category,
            commands::reorder_categories,
            // 任务命令
            commands::get_tasks,
            commands::add_task,
            commands::update_task,
            commands::delete_task,
            commands::reorder_tasks,
            commands::reset_task_sort,
            // 设置命令
            commands::get_settings,
            commands::update_settings,
            // 导出导入
            commands::export_data,
            commands::import_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}