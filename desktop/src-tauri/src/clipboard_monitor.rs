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

/// 延迟读取时间（毫秒）- 学习 Ditto：收到变化通知后延迟读取，避免干扰用户复制操作
const READ_DELAY_MS: u64 = 150;

pub struct ClipboardMonitor {
    running: Arc<AtomicBool>,
    /// 拖拽时设为 true，阻止监控器抓取自身写入的剪贴板内容
    skip_next: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            skip_next: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 标记下一次剪贴板变化应该被跳过（拖拽前调用）
    pub fn mark_skip_next(&self) {
        self.skip_next.store(true, Ordering::SeqCst);
    }

    pub fn start(&self, app_handle: tauri::AppHandle, db: Arc<Database>) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);

        let running = self.running.clone();
        let skip_next = self.skip_next.clone();

        thread::spawn(move || {
            let mut last_text_hash: Option<String> = None;
            let mut last_image_hash: Option<String> = None;
            // 使用更长的轮询间隔（3秒），减少剪贴板锁定频率
            let poll_interval = Duration::from_millis(3000);

            while running.load(Ordering::Relaxed) {
                // 跳过标记：跳过本次检查（拖拽后等待更长时间）
                if skip_next.swap(false, Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }

                // 关键优化：先快速检查剪贴板是否有新内容（不锁定）
                #[cfg(target_os = "windows")]
                {
                    // 使用 clipboard_win 的 is_format_avail 快速检查（不打开剪贴板）
                    use clipboard_win::formats;

                    let has_text = clipboard_win::raw::is_format_avail(formats::CF_UNICODETEXT)
                                || clipboard_win::raw::is_format_avail(formats::CF_TEXT);

                    if !has_text {
                        // 没有文本内容，检查是否有图片
                        let has_image = clipboard_win::raw::is_format_avail(formats::CF_DIB)
                                    || clipboard_win::raw::is_format_avail(formats::CF_DIBV5);

                        if !has_image {
                            // 既没有文本也没有图片，跳过这次读取
                            thread::sleep(poll_interval);
                            continue;
                        }
                    }
                }

                // 有内容变化，延迟一小段时间后再读取（学习 Ditto 的延迟机制）
                // 这让用户的复制操作有时间完成
                thread::sleep(Duration::from_millis(READ_DELAY_MS));

                // 快速读取：一次打开周期内同时读取文本和图像，立即释放
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
                            let result = db.add_auto_clipboard_image(&base64);
                            if result.is_ok() {
                                app_handle.emit("clipboard-changed", ()).ok();
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

                thread::sleep(poll_interval);
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

/// 尝试读取剪贴板中的文本（快速读取，立即释放）
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

/// 尝试读取剪贴板中的图片，返回 base64 data URL（快速读取，立即释放）
fn try_read_image() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // 优先使用 arboard 读取（最快路径）
        if let Some(b64) = read_via_arboard() {
            return Some(b64);
        }

        // arboard 失败后，再尝试 clipboard_win 的 DIB 格式
        use base64::{Engine, engine::general_purpose::STANDARD};
        use clipboard_win::{Clipboard, formats, raw};
        use image::codecs::bmp::BmpDecoder;
        use image::{ImageDecoder, DynamicImage};
        use std::io::Cursor;

        // 快速打开、读取、关闭剪贴板
        let clip = Clipboard::new().ok()?;

        // PNG 注册格式（浏览器/应用直接放 PNG 数据时）
        if let Some(png_fmt) = clipboard_win::register_format("PNG") {
            let mut data = Vec::new();
            if raw::get_vec(png_fmt.get(), &mut data).is_ok() && !data.is_empty() && data.len() < MAX_IMAGE_BASE64_LEN {
                return Some(format!("data:image/png;base64,{}", STANDARD.encode(&data)));
            }
        }

        // CF_DIBV5 格式
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 124 {
            if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                if width > 0 && height > 0 && width < 10000 && height < 10000 {
                    if let Ok(img) = DynamicImage::from_decoder(decoder) {
                        let mut png_buf = Vec::new();
                        if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                            && png_buf.len() >= 8
                            && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                        {
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        }
                    }
                }
            }
        }

        // CF_DIB 格式
        let mut data = Vec::new();
        if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() < 50_000_000 && data.len() >= 40 {
            if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
                let (width, height) = decoder.dimensions();
                if width > 0 && height > 0 && width < 10000 && height < 10000 {
                    if let Ok(img) = DynamicImage::from_decoder(decoder) {
                        let mut png_buf = Vec::new();
                        if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                            && png_buf.len() >= 8
                            && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                        {
                            return Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)));
                        }
                    }
                }
            }
        }

        // 注意：clip 在这里自动 drop，释放剪贴板锁
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

    None
}

/// 通过 arboard 读取 CF_BITMAP（Windows 最快路径）
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
    Some(format!("data:image/png;base64,{}", STANDARD.encode(&png_buf)))
}