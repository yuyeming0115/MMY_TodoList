use std::panic::catch_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::Emitter;

use crate::database::Database;

/// 最大剪贴板文本长度（1MB）
const MAX_TEXT_LEN: usize = 1_048_576;
/// 最大剪贴板图片 base64 长度（10MB）
const MAX_IMAGE_BASE64_LEN: usize = 10_485_760;

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
            let mut last_text_hash: Option<String> = None;
            let mut last_image_hash: Option<String> = None;

            while running.load(Ordering::Relaxed) {
                // 安全读取：用 catch_unwind 防止 native 代码 panic 导致整个进程崩溃
                let text_result = catch_unwind(|| try_read_text());
                let image_result = catch_unwind(|| try_read_image());

                // 1. 处理文本
                if let Ok(Some(text)) = text_result {
                    if text.len() <= MAX_TEXT_LEN {
                        let text_hash = hash_str(&text);
                        if last_text_hash.as_ref() != Some(&text_hash) {
                            if !db.clipboard_text_exists(&text).unwrap_or(true) {
                                if db.add_auto_clipboard_text(&text).is_ok() {
                                    app_handle.emit("clipboard-changed", ()).ok();
                                }
                            }
                            last_text_hash = Some(text_hash);
                        }
                    }
                }

                // 2. 处理图片
                if let Ok(Some(base64)) = image_result {
                    if base64.len() <= MAX_IMAGE_BASE64_LEN {
                        let img_hash = hash_str(&base64);
                        if last_image_hash.as_ref() != Some(&img_hash) {
                            if db.add_auto_clipboard_image(&base64).is_ok() {
                                app_handle.emit("clipboard-changed", ()).ok();
                            }
                            last_image_hash = Some(img_hash);
                        }
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

fn hash_str(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
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
        use image::codecs::bmp::BmpDecoder;
        use image::codecs::png::PngEncoder;
        use image::{ImageDecoder, ImageEncoder};
        use std::io::Cursor;

        let _clip = Clipboard::new().ok()?;

        // 方法一：PNG 注册格式
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            if is_format_avail(png_fmt.get()) {
                let mut data = Vec::new();
                if raw::get_vec(png_fmt.get(), &mut data).is_ok() && !data.is_empty() && data.len() < MAX_IMAGE_BASE64_LEN {
                    return Some(format!("data:image/png;base64,{}", STANDARD.encode(&data)));
                }
            }
        }

        // 方法二：CF_DIBV5
        if is_format_avail(formats::CF_DIBV5) {
            let mut data = Vec::new();
            if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 124 {
                if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                    let (width, height) = decoder.dimensions();
                    if width > 0 && height > 0 && width < 10000 && height < 10000 {
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

        // 方法三：CF_DIB
        if is_format_avail(formats::CF_DIB) {
            let mut data = Vec::new();
            if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 40 {
                if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                    let (width, height) = decoder.dimensions();
                    if width > 0 && height > 0 && width < 10000 && height < 10000 {
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
