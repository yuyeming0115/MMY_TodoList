use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::Emitter;

use crate::database::Database;

pub struct ClipboardMonitor {
    running: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, app_handle: tauri::AppHandle, db: Arc<Database>) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);

        let running = self.running.clone();

        thread::spawn(move || {
            let mut last_text: Option<String> = None;
            let mut last_image_hash: Option<String> = None;

            while running.load(Ordering::Relaxed) {
                // 1. 尝试读取文本
                if let Some(text) = try_read_text() {
                    // 内存去重
                    if last_text.as_ref() != Some(&text) {
                        // 数据库去重
                        if !db.clipboard_text_exists(&text).unwrap_or(true) {
                            if db.add_auto_clipboard_text(&text).is_ok() {
                                app_handle.emit("clipboard-changed", ()).ok();
                            }
                        }
                        last_text = Some(text);
                    }
                }

                // 2. 尝试读取图片
                if let Some(base64) = try_read_image() {
                    // 用 base64 前 100 字符做简单 hash 去重
                    let short_hash = if base64.len() > 100 {
                        &base64[..100]
                    } else {
                        &base64
                    };
                    let hash_str = short_hash.to_string();

                    if last_image_hash.as_ref() != Some(&hash_str) {
                        if db.add_auto_clipboard_image(&base64).is_ok() {
                            app_handle.emit("clipboard-changed", ()).ok();
                        }
                        last_image_hash = Some(hash_str);
                    }
                }

                thread::sleep(Duration::from_millis(1500));
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// 尝试读取剪贴板中的文本
fn try_read_text() -> Option<String> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        use arboard::Clipboard;
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}

/// 尝试读取剪贴板中的图片，返回 base64 data URL
fn try_read_image() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use clipboard_win::{Clipboard, formats, is_format_avail, raw};
        use image::codecs::png::PngEncoder;
        use image::codecs::bmp::BmpDecoder;
        use image::{ImageDecoder, ImageEncoder};
        use std::io::Cursor;

        let _clip = Clipboard::new().ok()?;

        // 方法一：PNG 注册格式
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            if is_format_avail(png_fmt.get()) {
                let mut data = Vec::new();
                if raw::get_vec(png_fmt.get(), &mut data).is_ok() && !data.is_empty() {
                    return Some(format!("data:image/png;base64,{}", STANDARD.encode(&data)));
                }
            }
        }

        // 方法二：CF_DIBV5
        if is_format_avail(formats::CF_DIBV5) {
            let mut data = Vec::new();
            if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() >= 124 {
                if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                    let (width, height) = decoder.dimensions();
                    if let Ok(img) = image::DynamicImage::from_decoder(decoder) {
                        let rgba = img.into_rgba8();
                        let mut png_buf = Vec::new();
                        if PngEncoder::new(&mut png_buf)
                            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
                            .is_ok()
                        {
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        }
                    }
                }
            }
        }

        // 方法三：CF_DIB
        if is_format_avail(formats::CF_DIB) {
            let mut data = Vec::new();
            if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() >= 40 {
                if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                    let (width, height) = decoder.dimensions();
                    if let Ok(img) = image::DynamicImage::from_decoder(decoder) {
                        let rgba = img.into_rgba8();
                        let mut png_buf = Vec::new();
                        if PngEncoder::new(&mut png_buf)
                            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
                            .is_ok()
                        {
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use arboard::Clipboard;
        use image::codecs::png::PngEncoder;
        use image::ImageEncoder;

        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(img_data) = clipboard.get_image() {
                let width = img_data.width as u32;
                let height = img_data.height as u32;
                let rgba_bytes: &[u8] = &img_data.bytes;
                let mut png_buf = Vec::new();
                if PngEncoder::new(&mut png_buf)
                    .write_image(rgba_bytes, width, height, image::ExtendedColorType::Rgba8)
                    .is_ok()
                {
                    return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                }
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {}

    None
}
