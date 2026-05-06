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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .on_menu_event(|app, event| {
            if event.id == "show" {
                if let Some(win) = app.get_webview_window("main") {
                    win.show().unwrap();
                    win.set_focus().unwrap();
                }
            } else if event.id == "quit" {
                app.exit(0);
                std::process::exit(0);
            }
        })
        .setup(|app| {
            // 初始化数据库
            let db = Database::init(&app.handle()).expect("数据库初始化失败");
            app.manage(db);
            // 初始化系统托盘
            setup_tray(app)?;

            // 监听窗口关闭事件，改为隐藏到托盘
            let app_handle = app.handle().clone();
            if let Some(win) = app.get_webview_window("main") {
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
            // 系统托盘
            hide_to_tray,
            // 工具命令
            find_pixpin_path,
            launch_pixpin,
            read_clipboard_image,
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
        use base64::{Engine, engine::general_purpose::STANDARD};
        use std::path::Path;
        use std::ptr::null_mut;
        use winapi::um::winuser::{
            OpenClipboard, CloseClipboard, GetClipboardData, EnumClipboardFormats,
            GetClipboardFormatNameW, CF_HDROP,
        };
        use winapi::um::winbase::GlobalSize;
        use winapi::um::shellapi::{DragQueryFileW};
        use winapi::shared::minwindef::MAX_PATH;

        for attempt in 0..10 {
            let result = (|| {
                unsafe {
                    // 打开剪贴板
                    if OpenClipboard(null_mut()) == 0 {
                        return Err("打开剪贴板失败".to_string());
                    }

                    // 枚举所有格式，打印调试信息
                    let mut fmt: u32 = 0;
                    loop {
                        fmt = EnumClipboardFormats(fmt);
                        if fmt == 0 { break; }
                        let mut name_buf = [0u16; 256];
                        let len = GetClipboardFormatNameW(fmt, name_buf.as_mut_ptr(), 255);
                        let name = if len > 0 {
                            String::from_utf16_lossy(&name_buf[..len as usize])
                        } else {
                            match fmt {
                                2 => "CF_BITMAP".into(),
                                8 => "CF_DIB".into(),
                                15 => "CF_HDROP".into(),
                                13 => "CF_UNICODETEXT".into(),
                                1 => "CF_TEXT".into(),
                                _ => format!("Format({})", fmt),
                            }
                        };

                        let hdata = GetClipboardData(fmt);
                        let info = if !hdata.is_null() {
                            let sz = GlobalSize(hdata);
                            // 如果是 CF_HDROP，读取文件路径
                            if fmt == CF_HDROP {
                                let count = DragQueryFileW(hdata as _, 0xFFFFFFFF, null_mut(), 0);
                                if count > 0 {
                                    let mut path = [0u16; MAX_PATH as usize * 4];
                                    let plen = DragQueryFileW(hdata as _, 0, path.as_mut_ptr(), path.len() as u32);
                                    if plen > 0 {
                                        let path_str = String::from_utf16_lossy(&path[..plen as usize]);
                                        format!("{} bytes, {} files, path: {}", sz, count, path_str)
                                    } else {
                                        format!("{} bytes, {} files", sz, count)
                                    }
                                } else {
                                    format!("{} bytes", sz)
                                }
                            } else {
                                format!("{} bytes", sz)
                            }
                        } else {
                            "null".into()
                        };

                        eprintln!("[CLIPBOARD] Format {}: {} - {}", fmt, name, info);
                    }

                    // 读取 CF_HDROP 文件路径
                    let hdrop = GetClipboardData(CF_HDROP);
                    if !hdrop.is_null() {
                        let count = DragQueryFileW(hdrop as _, 0xFFFFFFFF, null_mut(), 0);
                        if count > 0 {
                            let mut path = [0u16; MAX_PATH as usize * 4];
                            let plen = DragQueryFileW(hdrop as _, 0, path.as_mut_ptr(), path.len() as u32);
                            if plen > 0 {
                                let path_str = String::from_utf16_lossy(&path[..plen as usize]);
                                CloseClipboard();
                                let file_path = Path::new(&path_str);
                                if file_path.exists() {
                                    let ext = file_path.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("")
                                        .to_lowercase();
                                    let mime = match ext.as_str() {
                                        "jpg" | "jpeg" => "image/jpeg",
                                        "png" => "image/png",
                                        "gif" => "image/gif",
                                        "webp" => "image/webp",
                                        "bmp" => "image/bmp",
                                        _ => "image/png",
                                    };
                                    let file_data = std::fs::read(file_path)
                                        .map_err(|e| format!("读取文件失败: {}", e))?;
                                    return Ok(Some(format!("data:{};base64,{}", mime, STANDARD.encode(&file_data))));
                                }
                            }
                        }
                    }

                    CloseClipboard();
                    Ok::<Option<String>, String>(None)
                }
            })();

            match result {
                Ok(Some(base64)) => return Ok(Some(base64)),
                Ok(None) => return Ok(None),
                Err(e) => {
                    if e.contains("打开剪贴板失败") {
                        // 剪贴板被占用，等待重试
                        if attempt < 9 {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }
        Ok(None)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
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