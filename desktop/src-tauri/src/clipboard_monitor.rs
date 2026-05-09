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
                // 一次剪贴板打开周期内同时读取文本和图像，避免竞态
                let clip_text = catch_unwind(|| try_read_text());
                let clip_image = catch_unwind(|| try_read_image());

                // 1. 处理文本
                match clip_text {
                    Ok(Some(text)) if text.len() <= MAX_TEXT_LEN => {
                        let text_hash = hash_str(&text);
                        if last_text_hash.as_ref() != Some(&text_hash) {
                            let exists = db.clipboard_text_exists(&text).unwrap_or(false);
                            if !exists {
                                if db.add_auto_clipboard_text(&text).is_ok() {
                                    app_handle.emit("clipboard-changed", ()).ok();
                                }
                            }
                            last_text_hash = Some(text_hash);
                        }
                    }
                    Ok(Some(_)) => {} // 文本超长，跳过
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("[剪贴板] 文本读取 panic: {:?}", e);
                    }
                }

                // 2. 处理图像
                match clip_image {
                    Ok(Some(base64)) if base64.len() <= MAX_IMAGE_BASE64_LEN => {
                        let img_hash = hash_str(&base64);
                        if last_image_hash.as_ref() != Some(&img_hash) {
                            eprintln!("[剪贴板] 尝试保存图像 (hash: {}), base64长度: {}", &img_hash[..8], base64.len());
                            let result = db.add_auto_clipboard_image(&base64);
                            if result.is_ok() {
                                app_handle.emit("clipboard-changed", ()).ok();
                                eprintln!("[剪贴板] 图像保存成功");
                            } else {
                                eprintln!("[剪贴板] 图像保存失败: {:?}", result);
                            }
                            last_image_hash = Some(img_hash);
                        }
                    }
                    Ok(Some(_)) => {} // 图片超长，跳过
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("[剪贴板] 图像读取 panic: {:?}", e);
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
        // 优先使用 arboard 读取 CF_BITMAP（与 Mac 一致的路径）
        if let Some(b64) = read_via_arboard() {
            eprintln!("[剪贴板] arboard CF_BITMAP 成功");
            return Some(b64);
        }

        // arboard 失败后，再尝试 clipboard_win 的 DIB 格式
        use base64::{Engine, engine::general_purpose::STANDARD};
        use clipboard_win::{Clipboard, formats, raw};
        use image::codecs::bmp::BmpDecoder;
        use image::{ImageDecoder};
        use std::io::Cursor;

        let clip = Clipboard::new().ok()?;

        // 方法：PNG 注册格式（浏览器/应用直接放 PNG 数据时）
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            let mut data = Vec::new();
            if raw::get_vec(png_fmt.get(), &mut data).is_ok() && !data.is_empty() && data.len() < MAX_IMAGE_BASE64_LEN {
                eprintln!("[剪贴板] PNG 注册格式成功, {} bytes", data.len());
                return Some(format!("data:image/png;base64,{}", STANDARD.encode(&data)));
            }
        }

        // 方法：CF_DIBV5
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 124 {
            if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                if width > 0 && height > 0 && width < 10000 && height < 10000 {
                    if let Ok(img) = image::DynamicImage::from_decoder(decoder) {
                        // 使用 write_to 生成完整 PNG 文件（含文件头），而非 PngEncoder::write_image 只写裸像素
                        let mut png_buf = Vec::new();
                        if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                            && png_buf.len() >= 8
                            && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                        {
                            eprintln!("[剪贴板] CF_DIBV5 成功, {}x{}, {} bytes PNG", width, height, png_buf.len());
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        } else {
                            eprintln!("[剪贴板] CF_DIBV5: PNG 写入失败或文件头无效");
                        }
                    }
                }
            }
        }

        // 方法：CF_DIB
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 40 {
            if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                if width > 0 && height > 0 && width < 10000 && height < 10000 {
                    if let Ok(img) = image::DynamicImage::from_decoder(decoder) {
                        let mut png_buf = Vec::new();
                        if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                            && png_buf.len() >= 8
                            && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                        {
                            eprintln!("[剪贴板] CF_DIB 成功, {}x{}, {} bytes PNG", width, height, png_buf.len());
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        } else {
                            eprintln!("[剪贴板] CF_DIB: PNG 写入失败或文件头无效");
                        }
                    }
                }
            }
        }

        drop(clip);
        eprintln!("[剪贴板] 所有图像读取方法均失败");
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

/// 通过 arboard 读取 CF_BITMAP（Windows 主方案，与 Mac 一致）
#[cfg(target_os = "windows")]
fn read_via_arboard() -> Option<String> {
    use arboard::Clipboard;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use image::{DynamicImage, ImageFormat};

    let mut clipboard = Clipboard::new().ok()?;
    let img_data = clipboard.get_image().ok()?;
    let width = img_data.width as u32;
    let height = img_data.height as u32;
    let rgba = image::RgbaImage::from_raw(width, height, img_data.bytes.into_owned())?;
    let img = DynamicImage::ImageRgba8(rgba);

    let mut png_buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_buf), ImageFormat::Png).ok()?;

    eprintln!("[剪贴板] arboard CF_BITMAP 成功, {}x{}, {} bytes PNG", width, height, png_buf.len());
    Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)))
}

/// 验证 base64 字符串是否可正常解码（用于调试）
fn verify_base64(b64: &str) -> bool {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let stripped = if let Some(idx) = b64.find(",") { &b64[idx + 1..] } else { b64 };
    let result = STANDARD.decode(stripped);
    if let Err(e) = &result {
        eprintln!("[剪贴板] base64 验证失败: {:?}, 长度: {}", e, stripped.len());
    }
    result.is_ok()
}
