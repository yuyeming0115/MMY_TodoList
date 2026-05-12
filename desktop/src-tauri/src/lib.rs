mod models;
mod database;
mod commands;
mod clipboard_monitor;
mod backup;

use std::sync::Arc;

use database::Database;
use clipboard_monitor::ClipboardMonitor;
use backup::BackupManager;
use tauri::{
    menu::{MenuItem, MenuBuilder},
    tray::TrayIconEvent,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_menu_event(|app, event| {
            if event.id == "show" {
                if let Some(win) = app.get_webview_window("main") {
                    win.show().unwrap();
                    win.set_focus().unwrap();
                }
            } else if event.id == "quit" {
                // 退出前执行备份
                if let Some(backup_mgr) = app.try_state::<Arc<BackupManager>>() {
                    if backup_mgr.should_backup_on_close() {
                        backup_mgr.create_backup(app);
                    }
                }
                app.exit(0);
                std::process::exit(0);
            }
        })
        .setup(|app| {
            // 初始化数据库
            let db = Database::init(&app.handle()).expect("数据库初始化失败");
            let db_arc = Arc::new(db);
            app.manage(db_arc.clone());

            // 初始化备份管理器
            let backup_mgr = BackupManager::init(&app.handle(), db_arc.clone()).expect("备份管理器初始化失败");
            let backup_arc = Arc::new(backup_mgr);
            app.manage(backup_arc.clone());

            // 启动定时备份任务
            backup_arc.start_periodic_backup(app.handle().clone());

            // 启动剪贴板后台监控
            let monitor = ClipboardMonitor::new();
            let monitor_ref = &monitor;
            monitor_ref.start(app.handle().clone(), db_arc.clone());
            app.manage(monitor);

            // 初始化全局快捷键（从设置读取）
            init_global_shortcut(app, db_arc)?;

            // 初始化系统托盘
            setup_tray(app)?;

            // 监听窗口关闭事件，改为隐藏到托盘（同时备份）
            let app_handle = app.handle().clone();
            let backup_for_hide = backup_arc.clone();
            if let Some(win) = app.get_webview_window("main") {
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 隐藏前执行备份（如果启用）
                        if backup_for_hide.should_backup_on_close() {
                            backup_for_hide.create_backup(&app_handle);
                        }
                        // 阻止关闭，改为隐藏到托盘
                        api.prevent_close();
                        if let Some(win) = app_handle.get_webview_window("main") {
                            win.hide().unwrap();
                        }
                    }
                });
            }
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
            // 剪贴板分类
            commands::get_clipboard_categories,
            commands::add_clipboard_category,
            commands::update_clipboard_category,
            commands::delete_clipboard_category,
            commands::reorder_clipboard_categories,
            // 剪贴板项目
            commands::get_clipboard_items,
            commands::add_clipboard_item,
            commands::update_clipboard_item,
            commands::delete_clipboard_item,
            commands::reorder_clipboard_items,
            commands::read_clipboard_image_file,
            commands::set_clipboard_item_expiry,
            commands::cleanup_expired_items,
            // 跨应用拖拽
            commands::get_image_for_drag,
            commands::reveal_file_in_folder,
            commands::cleanup_invalid_image_items,
            // 备份命令
            commands::get_backup_settings,
            commands::update_backup_settings,
            commands::create_backup_now,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            // 系统托盘
            hide_to_tray,
            // 工具命令
            find_pixpin_path,
            launch_pixpin,
            read_clipboard_image,
            write_image_to_clipboard,
            // 快捷键命令
            commands::update_global_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 隐藏窗口到系统托盘
#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    // 隐藏前执行备份（如果启用）
    if let Some(backup_mgr) = app.try_state::<Arc<BackupManager>>() {
        if backup_mgr.should_backup_on_close() {
            backup_mgr.create_backup(&app);
        }
    }
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 查找运行中的 Pixpin 进程路径
#[tauri::command]
fn find_pixpin_path() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
            PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW,
            PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| format!("创建进程快照失败: {}", e))?;

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name = OsString::from_wide(
                        &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len())]
                    );
                    if name.to_string_lossy().to_lowercase() == "pixpin.exe" {
                        let hprocess = OpenProcess(
                            PROCESS_QUERY_LIMITED_INFORMATION,
                            false,
                            entry.th32ProcessID,
                        );
                        if let Ok(hprocess) = hprocess {
                            let mut buf = [0u16; MAX_PATH as usize * 4];
                            let mut len = buf.len() as u32;
                            if QueryFullProcessImageNameW(hprocess, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                                let path = OsString::from_wide(&buf[..len as usize])
                                    .to_string_lossy()
                                    .to_string();
                                CloseHandle(hprocess).ok();
                                CloseHandle(snapshot).ok();
                                return Ok(Some(path));
                            }
                            CloseHandle(hprocess).ok();
                        }
                    }
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            CloseHandle(snapshot).ok();
        }
        Ok(None)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

/// 启动 Pixpin 截图
#[tauri::command]
fn launch_pixpin(pixpin_path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new(&pixpin_path)
            .args(["-r", "pixpin.screenShotAndEdit()"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("启动 Pixpin 失败: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = pixpin_path;
        Err("不支持的平台".to_string())
    }
}

/// 读取剪贴板图片并转为 base64
#[tauri::command]
fn read_clipboard_image() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        for attempt in 0..10 {
            let result = try_read_once();
            match result {
                Ok(Some(base64)) => return Ok(Some(base64)),
                Ok(None) => return Ok(None),
                Err(e) => {
                    if e.to_lowercase().contains("no image") || e.to_lowercase().contains("content not available") {
                        return Ok(None);
                    }
                    if attempt < 9 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Ok(None)
    }

    #[cfg(target_os = "macos")]
    {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use arboard::Clipboard;
        use image::ImageEncoder;

        let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {}", e))?;

        if let Ok(img_data) = clipboard.get_image() {
            // arboard 返回的是 RGBA 数据，Cow<[u8]> 类型
            let width = img_data.width as u32;
            let height = img_data.height as u32;
            // 直接使用 Cow<[u8]> 的引用
            let rgba_bytes: &[u8] = &img_data.bytes;

            // 转换为 PNG 格式
            let mut png_buf = Vec::new();
            if image::codecs::png::PngEncoder::new(&mut png_buf)
                .write_image(rgba_bytes, width, height, image::ExtendedColorType::Rgba8)
                .is_ok()
            {
                return Ok(Some(STANDARD.encode(&png_buf)));
            }
        }
        Ok(None)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn try_read_once() -> Result<Option<String>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use clipboard_win::{Clipboard, formats, is_format_avail, raw};
    use image::ImageDecoder;
    use image::ImageEncoder;

    let _clip = Clipboard::new().map_err(|e| e.to_string())?;

    // 方法一：尝试读取 PNG 注册格式（PixPin 可能提供）
    if let Some(png_fmt) = clipboard_win::register_format("PNG") {
        if is_format_avail(png_fmt.get()) {
            let mut data = Vec::new();
            if raw::get_vec(png_fmt.get(), &mut data).is_ok() && !data.is_empty() {
                return Ok(Some(format!("data:image/png;base64,{}", STANDARD.encode(&data))));
            }
        }
    }

    // 方法二：尝试 CF_DIBV5（格式 17）
    if is_format_avail(formats::CF_DIBV5) {
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() >= 124 {
            if let Ok(decoder) = image::codecs::bmp::BmpDecoder::new_without_file_header(std::io::Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                let img = image::DynamicImage::from_decoder(decoder);
                if let Ok(img) = img {
                    let rgba = img.into_rgba8();
                    let mut png_buf = Vec::new();
                    if image::codecs::png::PngEncoder::new(&mut png_buf)
                        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
                        .is_ok()
                    {
                        return Ok(Some(STANDARD.encode(&png_buf)));
                    }
                }
            }
        }
    }

    // 方法三：尝试 CF_DIB（格式 8）
    if is_format_avail(formats::CF_DIB) {
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() >= 40 {
            if let Ok(decoder) = image::codecs::bmp::BmpDecoder::new_without_file_header(std::io::Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                let img = image::DynamicImage::from_decoder(decoder);
                if let Ok(img) = img {
                    let rgba = img.into_rgba8();
                    let mut png_buf = Vec::new();
                    if image::codecs::png::PngEncoder::new(&mut png_buf)
                        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
                        .is_ok()
                    {
                        return Ok(Some(STANDARD.encode(&png_buf)));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// 将 base64 图片写入系统剪贴板
#[tauri::command]
fn write_image_to_clipboard(base64: String) -> Result<(), String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let bytes = STANDARD.decode(&base64).map_err(|e| format!("Base64 解码失败: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        use clipboard_win::{Clipboard, formats, raw};
        use image::codecs::bmp::BmpEncoder;

        let img = image::load_from_memory(&bytes).map_err(|e| format!("图片解码失败: {}", e))?;
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();

        // 编码为 BMP（CF_DIB 格式）
        let mut bmp_buf = Vec::new();
        let mut encoder = BmpEncoder::new(&mut bmp_buf);
        encoder.encode(&rgba, w, h, image::ExtendedColorType::Rgba8)
            .map_err(|e| format!("BMP 编码失败: {}", e))?;

        // 去掉 BMP 文件头，保留 DIB 数据
        let dib_data = &bmp_buf[14..];

        let _clip = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))?;
        raw::set(formats::CF_DIB, dib_data).map_err(|e| format!("写入剪贴板失败: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;
        use image::load_from_memory;

        let img = load_from_memory(&bytes).map_err(|e| format!("图片解码失败: {}", e))?;
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();

        let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {}", e))?;
        clipboard.set_image(arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        }).map_err(|e| format!("写入剪贴板失败: {}", e))?;

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = bytes;
        Err("不支持的平台".to_string())
    }
}

/// 设置系统托盘菜单
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .item(&quit_item)
        .build()?;

    let _tray = app.tray_by_id("main")
        .expect("tray not found")
        .set_menu(Some(menu))?;

    Ok(())
}

/// 将窗口移动到鼠标位置附近并显示
pub(crate) fn show_window_at_cursor(app: &tauri::AppHandle) {
    use tauri::{Manager, Position, PhysicalPosition};

    if let Some(win) = app.get_webview_window("main") {
        // 先恢复窗口状态（如果最小化）
        if win.is_minimized().unwrap_or(false) {
            win.unminimize().ok();
        }

        // 获取鼠标位置
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            use windows::Win32::Foundation::POINT;

            unsafe {
                let mut point = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut point).is_ok() {
                    // 获取窗口大小
                    if let Ok(size) = win.outer_size() {
                        // 计算窗口位置（窗口中心在鼠标位置）
                        let x = point.x.saturating_sub(size.width as i32 / 2);
                        let y = point.y.saturating_sub(size.height as i32 / 2);

                        // 设置窗口位置
                        win.set_position(Position::Physical(PhysicalPosition { x, y })).ok();
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // 非 Windows 平台暂时不支持跟随鼠标
            // 未来可以添加 macOS 和 Linux 支持
        }

        win.show().ok();
        win.set_focus().ok();
    }
}

/// 初始化全局快捷键
fn init_global_shortcut(app: &tauri::App, db: Arc<Database>) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent};
    use tauri::Manager;

    // 从数据库读取快捷键设置
    let settings = db.get_settings()?;
    let shortcut_str = settings.global_shortcut.clone();

    if let Some(shortcut) = &shortcut_str {
        if !shortcut.is_empty() {
            // 注册快捷键并设置处理器
            let register_result = app.global_shortcut().on_shortcut(shortcut.as_str(), |app, _sc, event: ShortcutEvent| {
                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    if let Some(win) = app.get_webview_window("main") {
                        if win.is_minimized().unwrap_or(false) {
                            show_window_at_cursor(app);
                        } else if win.is_visible().unwrap_or(false) {
                            win.hide().ok();
                        } else {
                            show_window_at_cursor(app);
                        }
                    }
                }
            });

            if let Err(e) = register_result {
                eprintln!("[快捷键] 注册失败: {}，可能被其他应用占用，请在设置中更换", e);
                // 清除无效的快捷键设置
                let mut new_settings = settings;
                new_settings.global_shortcut = None;
                if let Err(e) = db.update_settings(&new_settings) {
                    eprintln!("[快捷键] 清除设置失败: {}", e);
                }
            }
        }
    }

    Ok(())
}