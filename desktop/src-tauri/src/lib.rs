mod models;
mod database;
mod commands;

use database::Database;
use tauri::{
    menu::{MenuItem, MenuBuilder},
    tray::TrayIconEvent,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 初始化数据库
            let db = Database::init(&app.handle()).expect("数据库初始化失败");
            app.manage(db);
            // 初始化系统托盘
            setup_tray(app)?;
            Ok(())
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button, .. } = event {
                if button == tauri::tray::MouseButton::Left {
                    if let Some(win) = tray.app_handle().get_webview_window("main") {
                        win.show().unwrap();
                        win.set_focus().unwrap();
                    }
                }
            }
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
            // 系统托盘
            hide_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 隐藏窗口到系统托盘
#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 设置系统托盘菜单
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::new(app, "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::new(app, "退出", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .item(&quit_item)
        .build()?;

    let _tray = app.tray_by_id("main")
        .expect("tray not found")
        .set_menu(Some(menu))?;

    // 监听菜单事件
    app.on_menu_event(|app, event| {
        if event.id == "显示窗口" {
            if let Some(win) = app.get_webview_window("main") {
                win.show().unwrap();
                win.set_focus().unwrap();
            }
        } else if event.id == "退出" {
            app.exit(0);
        }
    });

    Ok(())
}