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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二个实例启动时，激活第一个实例的窗口
            if let Some(win) = app.get_webview_window("main") {
                win.show().unwrap();
                win.set_focus().unwrap();
                win.unminimize().unwrap();
            }
        }))
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
                        backup_mgr.create_backup_default(app);
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

            // 启动剪贴板后台监控（根据设置决定是否启动）
            let monitor = ClipboardMonitor::new();
            let monitor_ref = &monitor;
            // 从设置读取是否启用剪贴板监控
            let settings = db_arc.get_settings().unwrap_or_default();
            let enable_clipboard_monitor = settings.enable_clipboard_monitor.unwrap_or(true);
            if enable_clipboard_monitor {
                monitor_ref.start(app.handle().clone(), db_arc.clone());
            }
            app.manage(monitor);

            // 初始化全局快捷键（从设置读取）
            init_global_shortcut(app, db_arc)?;

            // 初始化系统托盘
            setup_tray(app)?;

            // Mac 端：设置窗口背景为毛玻璃效果（Sidebar 材质）
            #[cfg(target_os = "macos")]
            {
                if let Some(win) = app.get_webview_window("main") {
                    window_vibrancy::apply_vibrancy(&win, window_vibrancy::NSVisualEffectMaterial::Sidebar, None, None)
                        .ok();
                }
            }

            // Windows 端：设置窗口圆角（消除角落白色像素）
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;
                use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;

                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(hwnd) = win.hwnd() {
                        unsafe {
                            // DWMWA_WINDOW_CORNER_PREFERENCE = 33
                            // DWMWCP_ROUND = 2 (中等圆角)
                            let corner_preference: u32 = 2; // DWMWCP_ROUND
                            DwmSetWindowAttribute(
                                hwnd,
                                DWMWINDOWATTRIBUTE(33), // DWMWA_WINDOW_CORNER_PREFERENCE
                                &corner_preference as *const u32 as *const std::ffi::c_void,
                                std::mem::size_of::<u32>() as u32,
                            ).ok();
                        }
                    }
                }
            }

            // 监听窗口关闭事件，改为隐藏到托盘（同时备份）
            let app_handle = app.handle().clone();
            let backup_for_hide = backup_arc.clone();
            if let Some(win) = app.get_webview_window("main") {
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 隐藏前执行备份（如果启用）
                        if backup_for_hide.should_backup_on_close() {
                            backup_for_hide.create_backup_default(&app_handle);
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
            commands::get_clipboard_items_paginated,
            commands::get_clipboard_items_count,
            commands::add_clipboard_item,
            commands::update_clipboard_item,
            commands::delete_clipboard_item,
            commands::reorder_clipboard_items,
            commands::batch_delete_clipboard_items,
            commands::batch_update_clipboard_items_category,
            commands::clear_all_unlocked_clipboard_items,
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
            commands::create_backup_with_type,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::preview_backup,
            commands::restore_backup_with_options,
            // 系统托盘
            hide_to_tray,
            // 剪贴板监控控制
            start_clipboard_monitor_cmd,
            stop_clipboard_monitor_cmd,
            // 工具命令
            find_pixpin_path,
            launch_pixpin,
            read_clipboard_image,
            write_image_to_clipboard,
            simulate_ctrl_v,
            mark_clipboard_skip_next,
            copy_image_from_path,
            copy_image_with_thumbnail,
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
            backup_mgr.create_backup_default(&app);
        }
    }
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 启动剪贴板监控
#[tauri::command]
fn start_clipboard_monitor_cmd(
    app: tauri::AppHandle,
    monitor: tauri::State<'_, ClipboardMonitor>,
    db: tauri::State<'_, Arc<Database>>,
) -> Result<(), String> {
    monitor.start(app, db.inner().clone());
    Ok(())
}

/// 停止剪贴板监控
#[tauri::command]
fn stop_clipboard_monitor_cmd(monitor: tauri::State<'_, ClipboardMonitor>) -> Result<(), String> {
    monitor.stop();
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

/// 将 base64 图片写入系统剪贴板（PNG 注册格式 + CF_DIB）
/// 异步执行：图片解码和编码在后台线程，避免阻塞 UI
#[tauri::command]
async fn write_image_to_clipboard(base64: String) -> Result<(), String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    // 在后台线程处理图片解码和编码（避免阻塞主线程）
    let clipboard_data = tauri::async_runtime::spawn_blocking(move || {
        let bytes = STANDARD.decode(&base64).map_err(|e| format!("Base64 解码失败: {}", e))?;

        #[cfg(target_os = "windows")]
        {
            use image::codecs::png::PngEncoder;
            use image::ImageEncoder;

            let img = image::load_from_memory(&bytes).map_err(|e| format!("图片解码失败: {}", e))?;
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();

            // PNG 编码
            let mut png_buf = Vec::new();
            PngEncoder::new(&mut png_buf)
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| format!("PNG 编码失败: {}", e))?;

            // 构造 CF_DIB 数据
            let dib_data = build_dib_data(&rgba, w, h);

            Ok::<(Vec<u8>, Vec<u8>), String>((png_buf, dib_data))
        }

        #[cfg(target_os = "macos")]
        {
            use image::load_from_memory;

            let img = load_from_memory(&bytes).map_err(|e| format!("图片解码失败: {}", e))?;
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();

            Ok::<(usize, usize, Vec<u8>), String>((w as usize, h as usize, rgba.into_raw()))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = bytes;
            Err::<Vec<u8>, String>("不支持的平台".to_string())
        }
    }).await.map_err(|e| format!("线程执行失败: {}", e))??;

    // 图片处理完成，现在写入剪贴板
    #[cfg(target_os = "windows")]
    {
        use clipboard_win::{Clipboard, raw};

        let (png_data, dib_data) = clipboard_data;
        let _clip = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))?;

        // 写入 PNG 注册格式（微信、浏览器、PS 都支持）
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            raw::set(png_fmt.get(), &png_data)
                .map_err(|e| format!("写入 PNG 失败: {}", e))?;
        }

        // 写入 CF_DIB 格式（传统 Windows 应用）
        raw::set(8, &dib_data)
            .map_err(|e| format!("写入 DIB 失败: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;

        let (w, h, raw_data) = clipboard_data;
        let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {}", e))?;
        clipboard.set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: std::borrow::Cow::Owned(raw_data),
        }).map_err(|e| format!("写入剪贴板失败: {}", e))?;

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok(())
    }
}

/// 标记下一次剪贴板变化应被跳过（拖拽前调用，防止监控器抓取自身写入）
#[tauri::command]
fn mark_clipboard_skip_next(monitor: tauri::State<'_, ClipboardMonitor>) {
    monitor.mark_skip_next();
}

/// 从文件路径直接复制图片到剪贴板（一步完成，避免前端多次调用）
/// 异步执行：文件读取、图片解码、PNG 编码都在后台线程
#[tauri::command]
async fn copy_image_from_path(path: String) -> Result<(), String> {
    // 在后台线程完成完整图片的处理
    let clipboard_data = tauri::async_runtime::spawn_blocking(move || {
        // 1. 读取文件
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("读取图片文件失败: {}", e))?;

        #[cfg(target_os = "windows")]
        {
            use image::codecs::png::PngEncoder;
            use image::ImageEncoder;

            // 2. 解码图片
            let img = image::load_from_memory(&bytes)
                .map_err(|e| format!("图片解码失败: {}", e))?;
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();

            // 3. PNG 编码
            let mut png_buf = Vec::new();
            PngEncoder::new(&mut png_buf)
                .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| format!("PNG 编码失败: {}", e))?;

            // 4. 构造 CF_DIB 数据（BITMAPINFOHEADER + BGRA 像素，从下到上）
            let dib_data = build_dib_data(&rgba, w, h);

            Ok::<(Vec<u8>, Vec<u8>), String>((png_buf, dib_data))
        }

        #[cfg(target_os = "macos")]
        {
            use image::load_from_memory;

            let img = load_from_memory(&bytes)
                .map_err(|e| format!("图片解码失败: {}", e))?;
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();

            Ok::<(usize, usize, Vec<u8>), String>((w as usize, h as usize, rgba.into_raw()))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = bytes;
            Err::<Vec<u8>, String>("不支持的平台".to_string())
        }
    }).await.map_err(|e| format!("线程执行失败: {}", e))??;

    // 5. 写入剪贴板（同时写入 PNG 和 CF_DIB 格式）
    #[cfg(target_os = "windows")]
    {
        use clipboard_win::{Clipboard, raw};

        let (png_data, dib_data) = clipboard_data;
        let _clip = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))?;

        // 写入 PNG 注册格式（现代应用：微信、浏览器、PS）
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            raw::set(png_fmt.get(), &png_data)
                .map_err(|e| format!("写入 PNG 失败: {}", e))?;
        }

        // 写入 CF_DIB 格式（格式号 8，传统 Windows 应用）
        raw::set(8, &dib_data)
            .map_err(|e| format!("写入 DIB 失败: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;

        let (w, h, raw_data) = clipboard_data;
        let mut clipboard = Clipboard::new().map_err(|e| format!("访问剪贴板失败: {}", e))?;
        clipboard.set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: std::borrow::Cow::Owned(raw_data),
        }).map_err(|e| format!("写入剪贴板失败: {}", e))?;
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok(())
    }
}

/// 复制图片到剪贴板（支持缩略图优先）
/// 大图：只写入缩略图（瞬间完成），避免阻塞用户的其他复制操作
/// 小图：直接写入完整图片
#[tauri::command]
async fn copy_image_with_thumbnail(path: String, thumbnail_base64: Option<String>) -> Result<(), String> {
    // 如果有缩略图，直接写入缩略图到剪贴板（瞬间完成，不阻塞）
    #[cfg(target_os = "windows")]
    if let Some(thumbnail) = thumbnail_base64 {
        write_thumbnail_to_clipboard(&thumbnail)?;
        return Ok(()); // 大图只写缩略图，立即返回
    }

    // 没有缩略图（小图），直接从文件复制
    copy_image_from_path(path).await
}

/// 快速写入缩略图到剪贴板（同步操作，瞬间完成）
#[cfg(target_os = "windows")]
fn write_thumbnail_to_clipboard(thumbnail_base64: &str) -> Result<(), String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use clipboard_win::{Clipboard, raw};
    use image::ImageEncoder;

    // 去掉 data:image/...;base64, 前缀
    let base64_data = if let Some(idx) = thumbnail_base64.find(",") {
        &thumbnail_base64[idx + 1..]
    } else {
        thumbnail_base64
    };

    let bytes = STANDARD.decode(base64_data)
        .map_err(|e| format!("缩略图解码失败: {}", e))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("缩略图图片解码失败: {}", e))?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();

    // 编码为 PNG
    let mut png_buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_buf)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("缩略图 PNG 编码失败: {}", e))?;

    // 构造 CF_DIB
    let dib_data = build_dib_data(&rgba, w, h);

    // 写入剪贴板
    let _clip = Clipboard::new().map_err(|e| format!("打开剪贴板失败: {}", e))?;

    if let Some(png_fmt) = clipboard_win::register_format("PNG") {
        raw::set(png_fmt.get(), &png_buf)
            .map_err(|e| format!("写入缩略图 PNG 失败: {}", e))?;
    }

    raw::set(8, &dib_data)
        .map_err(|e| format!("写入缩略图 DIB 失败: {}", e))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn write_thumbnail_to_clipboard(thumbnail_base64: &str) -> Result<(), String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use arboard::Clipboard;

    let base64_data = if let Some(idx) = thumbnail_base64.find(",") {
        &thumbnail_base64[idx + 1..]
    } else {
        thumbnail_base64
    };

    let bytes = STANDARD.decode(base64_data)
        .map_err(|e| format!("缩略图解码失败: {}", e))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("缩略图图片解码失败: {}", e))?;
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

/// 构造 CF_DIB 数据（BITMAPINFOHEADER + BGRA 像素，从下到上存储）
#[cfg(target_os = "windows")]
fn build_dib_data(rgba: &image::RgbaImage, width: u32, height: u32) -> Vec<u8> {
    // BITMAPINFOHEADER 结构（40 字节）
    let mut dib = Vec::with_capacity(40 + (width * height * 4) as usize);

    // biSize
    dib.extend_from_slice(&40u32.to_le_bytes());
    // biWidth
    dib.extend_from_slice(&width.to_le_bytes());
    // biHeight（正值表示从下到上）
    dib.extend_from_slice(&height.to_le_bytes());
    // biPlanes
    dib.extend_from_slice(&1u16.to_le_bytes());
    // biBitCount（32 位 = BGRA）
    dib.extend_from_slice(&32u16.to_le_bytes());
    // biCompression（BI_RGB = 0）
    dib.extend_from_slice(&0u32.to_le_bytes());
    // biSizeImage
    dib.extend_from_slice(&(width * height * 4).to_le_bytes());
    // biXPelsPerMeter
    dib.extend_from_slice(&0i32.to_le_bytes());
    // biYPelsPerMeter
    dib.extend_from_slice(&0i32.to_le_bytes());
    // biClrUsed
    dib.extend_from_slice(&0u32.to_le_bytes());
    // biClrImportant
    dib.extend_from_slice(&0u32.to_le_bytes());

    // 像素数据：从下到上，RGBA -> BGRA
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            let [r, g, b, a] = pixel.0;
            // BGRA 顺序
            dib.push(b);
            dib.push(g);
            dib.push(r);
            dib.push(a);
        }
    }

    dib
}

/// 模拟 Ctrl+V 粘贴（用于拖拽后自动粘贴）
#[tauri::command]
#[cfg(target_os = "windows")]
fn simulate_ctrl_v() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP,
        KEYBDINPUT, VIRTUAL_KEY,
    };

    // 等待目标窗口获得焦点
    std::thread::sleep(std::time::Duration::from_millis(200));

    unsafe {
        let mut inputs: [INPUT; 4] = std::mem::zeroed();

        // Ctrl 按下
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0x11), // VK_CONTROL
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };

        // V 按下
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0x56), // V
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };

        // V 释放
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0x56),
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        // Ctrl 释放
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0x11),
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent == 0 {
            return Err("发送按键事件失败".to_string());
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
fn simulate_ctrl_v() -> Result<(), String> {
    Err("仅支持 Windows 平台".to_string())
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
            use windows::Win32::Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromPoint, MONITOR_DEFAULTTONEAREST,
                MONITORINFOEXW, MONITORINFO,
            };
            use windows::Win32::Foundation::POINT;

            unsafe {
                let mut point = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut point).is_ok() {
                    // 获取鼠标所在显示器的工作区域（排除任务栏）
                    let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
                    let mut monitor_info = MONITORINFOEXW {
                        monitorInfo: MONITORINFO {
                            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                            ..Default::default()
                        },
                        szDevice: [0; 32],
                    };

                    let work_area = if GetMonitorInfoW(monitor, &mut monitor_info.monitorInfo).as_bool() {
                        monitor_info.monitorInfo.rcWork
                    } else {
                        // 如果获取失败，使用默认屏幕尺寸（保守估计）
                        windows::Win32::Foundation::RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
                    };

                    // 获取窗口大小
                    if let Ok(size) = win.outer_size() {
                        // 计算窗口位置（窗口中心在鼠标位置）
                        let x = point.x.saturating_sub(size.width as i32 / 2);
                        let y = point.y.saturating_sub(size.height as i32 / 2);

                        // 边界检测：确保窗口不超出显示器工作区域
                        // 右边界
                        let x = x.min(work_area.right - size.width as i32);
                        // 左边界
                        let x = x.max(work_area.left);
                        // 下边界
                        let y = y.min(work_area.bottom - size.height as i32);
                        // 上边界（关键：防止窗口顶部被遮挡）
                        let y = y.max(work_area.top);

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