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
/// 10M 像素约等于 3160x3160，5000x10000 = 50M 像素会触发此限制
const MAX_IMAGE_PIXELS_FOR_BASE64: u64 = 20_000_000;

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
                let clip_image = catch_unwind(|| {
                    // 先尝试 arboard 路径
                    try_read_image_data()
                        .or_else(|| try_read_image_via_clipboard_win())
                });

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
                    Ok(Some(img_data)) => {
                        eprintln!("[剪贴板] 成功读取图片: {}x{}, PNG数据 {} 字节",
                            img_data.width, img_data.height, img_data.png_data.len());

                        let pixels = img_data.width as u64 * img_data.height as u64;

                        // 计算图片 hash（基于 PNG 数据）
                        let img_hash = hash_bytes(&img_data.png_data);

                        if last_image_hash.as_ref() != Some(&img_hash) {
                            // 判断是否为大图片：超过像素限制，直接保存文件
                            let is_large_image = pixels > MAX_IMAGE_PIXELS_FOR_BASE64;
                            eprintln!("[剪贴板] 像素数: {}, 是否大图片: {}", pixels, is_large_image);

                            if is_large_image {
                                // 大图片：直接保存 PNG 文件，不转 base64
                                eprintln!("[剪贴板] 大图片处理: 直接保存文件...");
                                let result = db.add_auto_clipboard_image_large(
                                    &img_data.width,
                                    &img_data.height,
                                    &img_data.png_data,
                                );
                                if result.is_ok() {
                                    eprintln!("[剪贴板] 大图片保存成功!");
                                    app_handle.emit("clipboard-changed", ()).ok();
                                } else {
                                    eprintln!("[剪贴板] 大图片保存失败: {:?}", result.err());
                                }
                            } else {
                                // 小图片：转 base64 存入数据库
                                eprintln!("[剪贴板] 小图片处理: 转 base64...");
                                let base64 = format!(
                                    "data:image/png;base64,{}",
                                    base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &img_data.png_data
                                    )
                                );
                                eprintln!("[剪贴板] base64长度: {}", base64.len());
                                if base64.len() <= MAX_IMAGE_BASE64_LEN {
                                    let result = db.add_auto_clipboard_image(&base64);
                                    if result.is_ok() {
                                        eprintln!("[剪贴板] 小图片保存成功!");
                                        app_handle.emit("clipboard-changed", ()).ok();
                                    } else {
                                        eprintln!("[剪贴板] 小图片保存失败: {:?}", result.err());
                                    }
                                } else {
                                    eprintln!("[剪贴板] base64超长，跳过");
                                }
                            }
                            last_image_hash = Some(img_hash);
                        } else {
                            eprintln!("[剪贴板] 图片hash重复，跳过");
                        }
                    }
                    Ok(None) => {
                        eprintln!("[剪贴板] 未读取到图片");
                    }
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

fn hash_bytes(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
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

/// 剪贴板图片信息
struct ClipboardImageData {
    /// 图片宽度
    width: u32,
    /// 图片高度
    height: u32,
    /// PNG 数据（原始）
    png_data: Vec<u8>,
}

/// 尝试读取剪贴板中的图片，返回图片数据（快速读取，立即释放）
#[cfg(target_os = "windows")]
fn try_read_image_data() -> Option<ClipboardImageData> {
    // 优先使用 arboard 读取（最快路径）
    use arboard::Clipboard;
    use image::{DynamicImage, ImageFormat};

    let clipboard = Clipboard::new();
    if clipboard.is_err() {
        eprintln!("[剪贴板] arboard::Clipboard::new() 失败: {:?}", clipboard.err());
        return None;
    }
    let mut clipboard = clipboard.ok()?;

    let img_result = clipboard.get_image();
    if img_result.is_err() {
        eprintln!("[剪贴板] clipboard.get_image() 失败: {:?}", img_result.err());
        return None;
    }
    let img_data = img_result.ok()?;

    let width = img_data.width as u32;
    let height = img_data.height as u32;
    eprintln!("[剪贴板] 从 arboard 读取到图片: {}x{}, RGBA数据 {} 字节",
        width, height, img_data.bytes.len());

    // 检测尺寸：如果超大，直接返回原始 RGBA 数据让后续处理
    let rgba_result = image::RgbaImage::from_raw(width, height, img_data.bytes.into_owned());
    if rgba_result.is_none() {
        eprintln!("[剪贴板] RgbaImage::from_raw 失败，可能内存不足");
        return None;
    }
    let rgba = rgba_result?;
    let img = DynamicImage::ImageRgba8(rgba);

    let mut png_buf = Vec::new();
    let write_result = img.write_to(&mut std::io::Cursor::new(&mut png_buf), ImageFormat::Png);
    if write_result.is_err() {
        eprintln!("[剪贴板] PNG 编码失败: {:?}", write_result.err());
        return None;
    }

    eprintln!("[剪贴板] PNG 编码成功，数据大小: {} 字节", png_buf.len());

    Some(ClipboardImageData {
        width,
        height,
        png_data: png_buf,
    })
}

/// 尝试读取剪贴板中的图片（备用路径：clipboard_win）
#[cfg(target_os = "windows")]
fn try_read_image_via_clipboard_win() -> Option<ClipboardImageData> {
    use clipboard_win::{formats, raw};
    use image::codecs::bmp::BmpDecoder;
    use image::{ImageDecoder, DynamicImage};
    use std::io::Cursor;

    // CF_DIBV5 格式
    let mut data = Vec::new();
    if raw::get_vec(formats::CF_DIBV5, &mut data).is_ok() && !data.is_empty() && data.len() >= 124 {
        if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
            let (width, height) = decoder.dimensions();
            if width > 0 && height > 0 && width < 10000 && height < 10000 {
                if let Ok(img) = DynamicImage::from_decoder(decoder) {
                    let mut png_buf = Vec::new();
                    if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                        && png_buf.len() >= 8
                        && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                    {
                        return Some(ClipboardImageData {
                            width,
                            height,
                            png_data: png_buf,
                        });
                    }
                }
            }
        }
    }

    // CF_DIB 格式
    let mut data = Vec::new();
    if raw::get_vec(formats::CF_DIB, &mut data).is_ok() && !data.is_empty() && data.len() >= 40 {
        if let Ok(decoder) = BmpDecoder::new_without_file_header(Cursor::new(&data)) {
            let (width, height) = decoder.dimensions();
            if width > 0 && height > 0 && width < 10000 && height < 10000 {
                if let Ok(img) = DynamicImage::from_decoder(decoder) {
                    let mut png_buf = Vec::new();
                    if img.write_to(&mut Cursor::new(&mut png_buf), image::ImageFormat::Png).is_ok()
                        && png_buf.len() >= 8
                        && png_buf[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
                    {
                        return Some(ClipboardImageData {
                            width,
                            height,
                            png_data: png_buf,
                        });
                    }
                }
            }
        }
    }

    None
}

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