use crate::database::Database;
use crate::models::ExportData;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Manager;
use std::fs;
use std::io;
use base64::Engine;

/// 备份类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BackupType {
    /// 快速备份：仅元数据（不含图片 base64）
    Quick,
    /// 完整备份：包含图片 base64
    Full,
}

impl Default for BackupType {
    fn default() -> Self {
        Self::Quick
    }
}

/// 备份设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    /// 启用关闭时备份
    pub backup_on_close: bool,
    /// 启用每小时备份
    pub backup_hourly: bool,
    /// 保留天数（默认7天）
    pub retention_days: u32,
    /// 默认备份类型（快速/完整）
    pub default_backup_type: BackupType,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            backup_on_close: true,
            backup_hourly: false,
            retention_days: 7,
            default_backup_type: BackupType::Quick,
        }
    }
}

/// 备份文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: i64,
    pub size_bytes: u64,
    /// 备份类型（从文件名推断）
    pub backup_type: BackupType,
}

/// 备份管理器
pub struct BackupManager {
    settings: Mutex<BackupSettings>,
    db: Arc<Database>,
    backup_dir: PathBuf,
    periodic_running: AtomicBool,
}

impl BackupManager {
    pub fn init(app_handle: &tauri::AppHandle, db: Arc<Database>) -> io::Result<Self> {
        // 获取应用数据目录
        let app_dir = app_handle.path().app_data_dir()
            .expect("无法获取应用目录");

        // 创建备份目录
        let backup_dir = app_dir.join("backups");
        fs::create_dir_all(&backup_dir)?;

        // 加载备份设置
        let settings = Self::load_settings(&app_dir);

        Ok(Self {
            settings: Mutex::new(settings),
            db,
            backup_dir,
            periodic_running: AtomicBool::new(false),
        })
    }

    /// 加载备份设置
    fn load_settings(app_dir: &PathBuf) -> BackupSettings {
        let settings_path = app_dir.join("backup_settings.json");
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            BackupSettings::default()
        }
    }

    /// 获取当前设置（用于外部访问）
    pub fn get_settings(&self) -> BackupSettings {
        self.settings.lock().unwrap().clone()
    }

    /// 更新设置（内部可变性）
    pub fn update_settings_internal(&self, new_settings: BackupSettings) -> io::Result<()> {
        let mut settings = self.settings.lock().unwrap();
        *settings = new_settings.clone();
        self.save_settings_internal(&new_settings)?;
        Ok(())
    }

    /// 保存备份设置（内部使用）
    fn save_settings_internal(&self, settings: &BackupSettings) -> io::Result<()> {
        let app_dir = self.backup_dir.parent()
            .expect("无法获取父目录");
        let settings_path = app_dir.join("backup_settings.json");
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(&settings_path, content)?;
        Ok(())
    }

    /// 检查是否需要在关闭时备份
    pub fn should_backup_on_close(&self) -> bool {
        self.settings.lock().unwrap().backup_on_close
    }

    /// 创建备份（支持快速/完整备份类型）
    pub fn create_backup(&self, _app_handle: &tauri::AppHandle, backup_type: BackupType) -> Option<String> {
        let now = Utc::now();
        let type_suffix = match backup_type {
            BackupType::Quick => "_quick",
            BackupType::Full => "_full",
        };
        let filename = format!("backup_{}{}.mmytodo", now.format("%Y%m%d_%H%M%S"), type_suffix);
        let backup_path = self.backup_dir.join(&filename);

        // 导出数据
        let categories = self.db.get_categories().ok()?;
        let tasks = self.db.get_tasks().ok()?;
        let clipboard_categories = self.db.get_clipboard_categories().ok()?;
        let mut clipboard_items = self.db.get_clipboard_items().ok()?;
        let settings = self.db.get_settings().ok()?;

        // 根据备份类型处理图片
        match backup_type {
            BackupType::Quick => {
                // 快速备份：不转换图片为 base64，只保留路径引用
                // 图片目录需要单独备份或后续恢复时从其他来源获取
            }
            BackupType::Full => {
                // 完整备份：将 image_path 的图片转成 base64 存入 JSON
                for item in &mut clipboard_items {
                    if let Some(path) = &item.image_path {
                        // 读取图片文件转成 base64
                        if let Ok(data) = fs::read(path) {
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                            item.image_base64 = Some(format!("data:image/png;base64,{}", b64));
                        }
                    }
                }
            }
        }

        let export_data = ExportData {
            version: "3.0".to_string(),
            exported_at: now.to_rfc3339(),
            source: match backup_type {
                BackupType::Quick => "backup_quick".to_string(),
                BackupType::Full => "backup_full".to_string(),
            },
            categories,
            tasks,
            clipboard_categories,
            clipboard_items,
            settings,
        };

        // 写入备份文件
        let json = serde_json::to_string_pretty(&export_data).ok()?;
        fs::write(&backup_path, &json).ok()?;

        // 清理过期备份
        self.cleanup_old_backups();

        Some(filename)
    }

    /// 创建默认类型备份（使用设置中的默认备份类型）
    pub fn create_backup_default(&self, app_handle: &tauri::AppHandle) -> Option<String> {
        let backup_type = self.settings.lock().unwrap().default_backup_type.clone();
        self.create_backup(app_handle, backup_type)
    }

    /// 清理超过保留天数的备份，并确保最多只保留7个
    fn cleanup_old_backups(&self) {
        const MAX_BACKUPS: usize = 7;

        let retention_days = self.settings.lock().unwrap().retention_days;
        let retention_ms = (retention_days as i64) * 24 * 60 * 60 * 1000;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 收集所有备份文件及其时间
        let mut backups: Vec<(PathBuf, i64)> = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "mmytodo").unwrap_or(false) {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if filename.starts_with("backup_") {
                        if let Some((backup_time, _)) = Self::parse_backup_time(filename) {
                            backups.push((path, backup_time));
                        }
                    }
                }
            }
        }

        // 按时间倒序排序（最新的在前）
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // 先删除过期的备份
        for (path, time) in backups.iter() {
            if now - *time > retention_ms {
                fs::remove_file(path).ok();
            }
        }

        // 更新列表（移除已删除的）
        let remaining: Vec<_> = backups
            .iter()
            .filter(|(path, _)| path.exists())
            .collect();

        // 如果超过7个，删除最旧的
        if remaining.len() > MAX_BACKUPS {
            for (path, _) in remaining.iter().skip(MAX_BACKUPS) {
                fs::remove_file(path).ok();
            }
        }
    }

    /// 解析备份时间戳（支持带 _quick/_full 后缀的格式）
    fn parse_backup_time(filename: &str) -> Option<(i64, BackupType)> {
        // 格式: backup_YYYYMMDD_HHMMSS_quick.mmytodo 或 backup_YYYYMMDD_HHMMSS_full.mmytodo
        // 或旧格式: backup_YYYYMMDD_HHMMSS.mmytodo

        // 去掉 .mmytodo 后缀
        let name_without_ext = filename.strip_suffix(".mmytodo")?;

        // 去掉 backup_ 前缀
        let rest = name_without_ext.strip_prefix("backup_")?;

        // 检查是否有类型后缀
        let (timestamp_str, backup_type) = if let Some(ts) = rest.strip_suffix("_quick") {
            (ts, BackupType::Quick)
        } else if let Some(ts) = rest.strip_suffix("_full") {
            (ts, BackupType::Full)
        } else {
            // 旧格式，默认为 Full（因为包含图片）
            (rest, BackupType::Full)
        };

        // 解析时间戳 YYYYMMDD_HHMMSS
        if timestamp_str.len() != 15 {
            return None;
        }

        let year: i32 = timestamp_str[0..4].parse().ok()?;
        let month: u32 = timestamp_str[4..6].parse().ok()?;
        let day: u32 = timestamp_str[6..8].parse().ok()?;
        let hour: u32 = timestamp_str[9..11].parse().ok()?;
        let min: u32 = timestamp_str[11..13].parse().ok()?;
        let sec: u32 = timestamp_str[13..15].parse().ok()?;

        let created_at = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, min, sec))
            .map(|dt| dt.and_utc().timestamp_millis())?;

        Some((created_at, backup_type))
    }

    /// 列出所有备份
    pub fn list_backups(&self) -> Vec<BackupInfo> {
        let mut backups = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "mmytodo").unwrap_or(false) {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    let metadata = entry.metadata().ok();
                    let size_bytes = metadata.map(|m| m.len()).unwrap_or(0);

                    // 解析创建时间和备份类型
                    let (created_at, backup_type) = Self::parse_backup_time(&filename)
                        .unwrap_or((0, BackupType::Full));

                    backups.push(BackupInfo {
                        filename,
                        created_at,
                        size_bytes,
                        backup_type,
                    });
                }
            }
        }

        // 按时间倒序排序（最新的在前）
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        backups
    }

    /// 恢复备份
    pub fn restore_backup(&self, filename: &str) -> io::Result<()> {
        let backup_path = self.backup_dir.join(filename);
        if !backup_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "备份文件不存在"));
        }

        let content = fs::read_to_string(&backup_path)?;
        let data: ExportData = serde_json::from_str(&content)?;

        // 清空现有数据
        {
            let conn = self.db.conn.lock().unwrap();
            conn.execute("DELETE FROM categories", []).ok();
            conn.execute("DELETE FROM tasks", []).ok();
            conn.execute("DELETE FROM clipboard_categories", []).ok();
            conn.execute("DELETE FROM clipboard_items", []).ok();
        }

        // 导入分类（使用原有 ID）
        for category in &data.categories {
            self.db.import_category(category).ok();
        }

        // 导入任务（使用原有 ID 和 sort_order）
        for task in &data.tasks {
            self.db.import_task(task).ok();
        }

        // 导入剪贴板分类（使用原有 ID）
        for category in &data.clipboard_categories {
            self.db.import_clipboard_category(category).ok();
        }

        // 导入剪贴板项目（恢复图片文件）
        for item in &data.clipboard_items {
            let mut item = item.clone();
            // 如果有 base64 数据，重新保存图片文件
            if item.image_base64.is_some() {
                let result = self.db.save_clipboard_image_file(&item.id, &item.image_base64.clone().unwrap());
                if let Ok(path) = result {
                    item.image_path = Some(path);
                }
            }
            self.db.add_clipboard_item(&item).ok();
        }

        // 导入设置
        self.db.update_settings(&data.settings).ok();

        Ok(())
    }

    /// 删除备份
    pub fn delete_backup(&self, filename: &str) -> io::Result<()> {
        let backup_path = self.backup_dir.join(filename);
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }
        Ok(())
    }

    /// 启动定时备份任务
    pub fn start_periodic_backup(&self, _app_handle: tauri::AppHandle) {
        if self.periodic_running.load(Ordering::SeqCst) {
            return;
        }
        self.periodic_running.store(true, Ordering::SeqCst);

        let settings = self.get_settings();
        let backup_dir = self.backup_dir.clone();
        let db = self.db.clone();

        std::thread::spawn(move || {
            let mut last_hourly_backup: i64 = 0;

            loop {
                std::thread::sleep(std::time::Duration::from_secs(60)); // 每分钟检查一次

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;

                // 每小时备份
                if settings.backup_hourly && now - last_hourly_backup >= 3600000 {
                    last_hourly_backup = now;
                    Self::do_backup(&backup_dir, &db);
                }
            }
        });
    }

    fn do_backup(backup_dir: &PathBuf, db: &Arc<Database>) {
        let now = Utc::now();
        let filename = format!("backup_{}.mmytodo", now.format("%Y%m%d_%H%M%S"));
        let backup_path = backup_dir.join(&filename);

        if let (Ok(categories), Ok(tasks), Ok(clipboard_categories), Ok(clipboard_items), Ok(settings)) = (
            db.get_categories(),
            db.get_tasks(),
            db.get_clipboard_categories(),
            db.get_clipboard_items(),
            db.get_settings(),
        ) {
            let export_data = ExportData {
                version: "3.0".to_string(),
                exported_at: now.to_rfc3339(),
                source: "backup".to_string(),
                categories,
                tasks,
                clipboard_categories,
                clipboard_items,
                settings,
            };

            if let Ok(json) = serde_json::to_string_pretty(&export_data) {
                if fs::write(&backup_path, &json).is_ok() {
                    // 清理过期备份
                    Self::cleanup_old_backups_static(backup_dir, 7);
                }
            }
        }
    }

    fn cleanup_old_backups_static(backup_dir: &PathBuf, retention_days: u32) {
        let retention_ms = (retention_days as i64) * 24 * 60 * 60 * 1000;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        if let Ok(entries) = fs::read_dir(backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "mmytodo").unwrap_or(false) {
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if filename.starts_with("backup_") {
                        if let Some((backup_time, _)) = Self::parse_backup_time(filename) {
                            if now - backup_time > retention_ms {
                                fs::remove_file(&path).ok();
                            }
                        }
                    }
                }
            }
        }
    }
}