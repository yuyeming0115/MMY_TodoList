use std::panic::catch_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::Emitter;

use crate::database::Database;

/// 最大剪贴板文本长度（1MB）
const MAX_TEXT_LEN: usize = 1_048_576;
/// 最大剪贴板图片 base64 长度（50MB）- 用于小图片
const MAX_IMAGE_BASE64_LEN: usize = 52_428_800;
/// 最大图片像素数（超过此值只存路径，不转 base64）
const MAX_IMAGE_PIXELS_FOR_BASE64: u64 = 20_000_000;
/// 缩略图最大尺寸（像素）
const THUMBNAIL_MAX_SIZE: u32 = 200;

/// 延迟读取时间（毫秒）- 收到变化通知后延迟读取，避免干扰用户复制操作
const READ_DELAY_MS: u64 = 100;

/// 轮询间隔（毫秒）- 短轮询以减少遗漏几率
const POLL_INTERVAL_MS: u64 = 500;

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
            start_clipboard_monitor(running, skip_next, app_handle, db);
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// 启动剪贴板监控（跨平台）
fn start_clipboard_monitor(
    running: Arc<AtomicBool>,
    skip_next: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,
    db: Arc<Database>,
) {
    let mut last_text_hash: Option<String> = None;
    let mut last_image_hash: Option<String> = None;
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);

    eprintln!("[剪贴板] 开始监控，轮询间隔: {}ms", POLL_INTERVAL_MS);

    while running.load(Ordering::Relaxed) {
        // 跳过标记：跳过本次检查（拖拽操作后）
        if skip_next.swap(false, Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1000));
            continue;
        }

        // 延迟一小段时间让用户的复制操作完成
        thread::sleep(Duration::from_millis(READ_DELAY_MS));

        // 读取剪贴板内容
        read_and_process_clipboard(&db, &app_handle, &mut last_text_hash, &mut last_image_hash);

        thread::sleep(poll_interval);
    }

    eprintln!("[剪贴板] 监控已停止");
}

/// 读取并处理剪贴板内容
fn read_and_process_clipboard(
    db: &Arc<Database>,
    app_handle: &tauri::AppHandle,
    last_text_hash: &mut Option<String>,
    last_image_hash: &mut Option<String>,
) {
    #[cfg(target_os = "windows")]
    {
        use clipboard_win::Clipboard;
        use clipboard_win::{formats, raw};

        // 尝试打开剪贴板（最多10次）
        let clipboard_result = Clipboard::new_attempts(10);
        if clipboard_result.is_err() {
            return;
        }
        let _clipboard = clipboard_result.unwrap();

        // 读取文本
        let clip_text = catch_unwind(|| {
            let mut data = Vec::new();
            if raw::get_vec(formats::CF_UNICODETEXT, &mut data).is_ok() && !data.is_empty() && data.len() >= 2 {
                let len = if data[data.len() - 2] == 0 && data[data.len() - 1] == 0 {
                    data.len() - 2
                } else {
                    data.len()
                };
                let utf16_chars: Vec<u16> = data[..len].chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&utf16_chars).ok().filter(|t| !t.is_empty())
            } else {
                let mut data = Vec::new();
                if raw::get_vec(formats::CF_TEXT, &mut data).is_ok() && !data.is_empty() && data.len() >= 1 {
                    let len = if data[data.len() - 1] == 0 { data.len() - 1 } else { data.len() };
                    String::from_utf8(data[..len].to_vec()).ok().filter(|t| !t.is_empty())
                } else {
                    None
                }
            }
        });

        // 读取图片
        let clip_image = catch_unwind(|| try_read_image_via_raw());

        process_clipboard_data(clip_text, clip_image, db, app_handle, last_text_hash, last_image_hash);
    }

    #[cfg(target_os = "macos")]
    {
        let clip_text = catch_unwind(|| try_read_text());
        let clip_image = catch_unwind(|| try_read_image_data());
        process_clipboard_data(clip_text, clip_image, db, app_handle, last_text_hash, last_image_hash);
    }
}

fn hash_str(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn hash_bytes(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// 剪贴板图片信息
struct ClipboardImageData {
    width: u32,
    height: u32,
    png_data: Vec<u8>,
}

/// macOS: 使用 arboard 读取图片
#[cfg(target_os = "macos")]
fn try_read_image_data() -> Option<ClipboardImageData> {
    use arboard::Clipboard;
    use image::{DynamicImage, ImageFormat};

    let mut clipboard = Clipboard::new().ok()?;
    let img_data = clipboard.get_image().ok()?;
    let width = img_data.width as u32;
    let height = img_data.height as u32;
    let rgba = image::RgbaImage::from_raw(width, height, img_data.bytes.into_owned())?;
    let img = DynamicImage::ImageRgba8(rgba);

    let mut png_buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_buf), ImageFormat::Png).ok()?;

    Some(ClipboardImageData {
        width,
        height,
        png_data: png_buf,
    })
}

/// macOS: 使用 arboard 读取文本
#[cfg(target_os = "macos")]
fn try_read_text() -> Option<String> {
    use arboard::Clipboard;
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Windows: 使用 raw API 读取图片（剪贴板已打开）
#[cfg(target_os = "windows")]
fn try_read_image_via_raw() -> Option<ClipboardImageData> {
    use clipboard_win::{formats, raw};
    use image::codecs::bmp::BmpDecoder;
    use image::{DynamicImage, ImageDecoder};
    use std::io::Cursor;

    // 尝试 CF_DIBV5
    let mut data = Vec::new();
    if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() >= 124 {
        if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
            let (width, height) = decoder.dimensions();
            if width > 0 && height > 0 && width < 50000 && height < 50000 {
                if let Ok(img) = DynamicImage::from_decoder(decoder) {
                    let mut png_buf = Vec::new();
                    if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                        && png_buf.len() >= 8
                        && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                        return Some(ClipboardImageData { width, height, png_data: png_buf });
                    }
                }
            }
        }
    }

    // 尝试 CF_DIB
    let mut data = Vec::new();
    if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() >= 40 {
        if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
            let (width, height) = decoder.dimensions();
            if width > 0 && height > 0 && width < 50000 && height < 50000 {
                if let Ok(img) = DynamicImage::from_decoder(decoder) {
                    let mut png_buf = Vec::new();
                    if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                        && png_buf.len() >= 8
                        && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                        return Some(ClipboardImageData { width, height, png_data: png_buf });
                    }
                }
            }
        }
    }

    None
}

/// 处理剪贴板数据（通用处理逻辑）
fn process_clipboard_data(
    clip_text: std::thread::Result<Option<String>>,
    clip_image: std::thread::Result<Option<ClipboardImageData>>,
    db: &Arc<Database>,
    app_handle: &tauri::AppHandle,
    last_text_hash: &mut Option<String>,
    last_image_hash: &mut Option<String>,
) {
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
                *last_text_hash = Some(text_hash);
            }
        }
        Ok(Some(_)) | Ok(None) | Err(_) => {}
    }

    // 2. 处理图像
    match clip_image {
        Ok(Some(img_data)) => {
            let pixels = img_data.width as u64 * img_data.height as u64;
            let img_hash = hash_bytes(&img_data.png_data);

            if last_image_hash.as_ref() != Some(&img_hash) {
                let is_large_image = pixels > MAX_IMAGE_PIXELS_FOR_BASE64;

                if is_large_image {
                    // 大图处理：移到后台线程，避免阻塞剪贴板监控
                    let db_clone = db.clone();
                    let app_handle_clone = app_handle.clone();
                    let png_data = img_data.png_data.clone();
                    let width = img_data.width;
                    let height = img_data.height;

                    thread::spawn(move || {
                        // 生成缩略图（耗时操作，在后台线程执行）
                        let thumbnail = generate_thumbnail(&png_data, THUMBNAIL_MAX_SIZE);
                        if db_clone.add_auto_clipboard_image_large_with_thumbnail(
                            &width,
                            &height,
                            &png_data,
                            thumbnail.as_deref(),
                        ).is_ok() {
                            app_handle_clone.emit("clipboard-changed", ()).ok();
                        }
                    });
                } else {
                    let base64 = format!(
                        "data:image/png;base64,{}",
                        base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &img_data.png_data
                        )
                    );
                    if base64.len() <= MAX_IMAGE_BASE64_LEN {
                        if db.add_auto_clipboard_image(&base64).is_ok() {
                            app_handle.emit("clipboard-changed", ()).ok();
                        }
                    }
                }
                *last_image_hash = Some(img_hash);
            }
        }
        Ok(None) | Err(_) => {}
    }
}

/// 生成缩略图（用于大图片快速复制）
/// 将图片缩放到指定最大尺寸，返回 base64 编码的 PNG
fn generate_thumbnail(png_data: &[u8], max_size: u32) -> Option<String> {
    use image::{ImageFormat, imageops, GenericImageView};
    use base64::{Engine, engine::general_purpose::STANDARD};

    let img = image::load_from_memory(png_data).ok()?;
    let (w, h) = img.dimensions();

    // 计算缩略图尺寸（保持比例）
    let max_dim = w.max(h);
    if max_dim <= max_size {
        // 图片已经足够小，直接编码
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png).ok()?;
        return Some(format!("data:image/png;base64,{}", STANDARD.encode(&buf)));
    }

    let ratio = max_size as f32 / max_dim as f32;
    let thumb_w = (w as f32 * ratio) as u32;
    let thumb_h = (h as f32 * ratio) as u32;

    // 缩放图片（使用 Triangle 滤波器，速度快）
    let thumbnail = img.resize(thumb_w, thumb_h, imageops::FilterType::Triangle);

    // 编码为 PNG base64
    let mut buf = Vec::new();
    thumbnail.write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png).ok()?;

    Some(format!("data:image/png;base64,{}", STANDARD.encode(&buf)))
}