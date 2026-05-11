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

/// 备份设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    /// 启用每日备份
    pub backup_daily: bool,
    /// 启用关闭时备份
    pub backup_on_close: bool,
    /// 启用每小时备份
    pub backup_hourly: bool,
    /// 保留天数（默认7天）
    pub retention_days: u32,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            backup_daily: true,
            backup_on_close: true,
            backup_hourly: false,
            retention_days: 7,
        }
    }
}

/// 备份文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: i64,
    pub size_bytes: u64,
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

    /// 创建备份
    pub fn create_backup(&self, _app_handle: &tauri::AppHandle) -> Option<String> {
        let now = Utc::now();
        let filename = format!("backup_{}.mmytodo", now.format("%Y%m%d_%H%M%S"));
        let backup_path = self.backup_dir.join(&filename);

        // 导出数据
        let categories = self.db.get_categories().ok()?;
        let tasks = self.db.get_tasks().ok()?;
        let clipboard_categories = self.db.get_clipboard_categories().ok()?;
        let clipboard_items = self.db.get_clipboard_items().ok()?;
        let settings = self.db.get_settings().ok()?;

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

        // 写入备份文件
        let json = serde_json::to_string_pretty(&export_data).ok()?;
        fs::write(&backup_path, &json).ok()?;

        // 清理过期备份
        self.cleanup_old_backups();

        Some(filename)
    }

    /// 清理超过保留天数的备份
    fn cleanup_old_backups(&self) {
        let retention_days = self.settings.lock().unwrap().retention_days;
        let retention_ms = (retention_days as i64) * 24 * 60 * 60 * 1000;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "mmytodo").unwrap_or(false) {
                    // 从文件名解析时间
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // 格式: backup_YYYYMMDD_HHMMSS.mmytodo
                    if filename.starts_with("backup_") {
                        let timestamp_str = &filename[7..filename.len() - 7]; // 去掉 "backup_" 和 ".mmytodo"
                        // 解析 YYYYMMDD_HHMMSS
                        if let Some(backup_time) = Self::parse_backup_time(timestamp_str) {
                            if now - backup_time > retention_ms {
                                fs::remove_file(&path).ok();
                            }
                        }
                    }
                }
            }
        }
    }

    /// 解析备份时间戳
    fn parse_backup_time(timestamp_str: &str) -> Option<i64> {
        // 格式: YYYYMMDD_HHMMSS
        if timestamp_str.len() != 15 {
            return None;
        }
        let year: i32 = timestamp_str[0..4].parse().ok()?;
        let month: u32 = timestamp_str[4..6].parse().ok()?;
        let day: u32 = timestamp_str[6..8].parse().ok()?;
        let hour: u32 = timestamp_str[9..11].parse().ok()?;
        let min: u32 = timestamp_str[11..13].parse().ok()?;
        let sec: u32 = timestamp_str[13..15].parse().ok()?;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, min, sec))
            .map(|dt| dt.and_utc().timestamp_millis())
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

                    // 解析创建时间
                    let created_at = Self::parse_backup_time(
                        &filename[7..filename.len() - 7]
                    ).unwrap_or(0);

                    backups.push(BackupInfo {
                        filename,
                        created_at,
                        size_bytes,
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

        // 导入分类
        for category in &data.categories {
            self.db.add_category(category.name.clone(), category.color.clone()).ok();
        }

        // 导入任务
        for task in &data.tasks {
            self.db.add_task(task).ok();
        }

        // 导入剪贴板分类
        for category in &data.clipboard_categories {
            self.db.add_clipboard_category(category.name.clone(), category.color.clone()).ok();
        }

        // 导入剪贴板项目
        for item in &data.clipboard_items {
            self.db.add_clipboard_item(item).ok();
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
            let mut last_daily_backup: i64 = 0;
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

                // 每日备份（凌晨2点执行）
                if settings.backup_daily {
                    let now_dt = chrono::DateTime::from_timestamp_millis(now)
                        .unwrap_or_else(|| Utc::now());
                    let today_2am = now_dt.date_naive()
                        .and_hms_opt(2, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp_millis();

                    if now >= today_2am && last_daily_backup < today_2am {
                        last_daily_backup = now;
                        Self::do_backup(&backup_dir, &db);
                    }
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
                        let timestamp_str = &filename[7..filename.len() - 7];
                        if let Some(backup_time) = Self::parse_backup_time(timestamp_str) {
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