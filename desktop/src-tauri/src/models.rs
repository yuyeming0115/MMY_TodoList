use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
    pub created_at: i64,
}

impl Category {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            sort_order: 0,
            created_at: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub category_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<i64>,
    pub due_date: Option<i64>,
    pub priority: i32,
    pub status: String,
    pub sort_order: i32,
    pub is_pinned: bool, // 置顶
    pub thumbnail_base64: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Task {
    pub fn new(
        category_id: String,
        title: String,
        description: Option<String>,
        start_date: Option<i64>,
        due_date: Option<i64>,
        priority: i32,
        thumbnail_base64: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            category_id,
            title,
            description,
            start_date,
            due_date,
            priority,
            status: "todo".to_string(),
            sort_order: 0,
            is_pinned: false,
            thumbnail_base64,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme_mode: String,
    pub language: String,
    pub hide_completed_tasks: bool,
    pub launch_at_startup: bool,
    pub window_width: Option<i32>,
    pub window_height: Option<i32>,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub font_size: i32,
    pub font_family: String,
    pub clipboard_view_mode: String,
    pub clipboard_stack_gap: i32, // 层叠模式卡片间距（px）
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_mode: "system".to_string(),
            language: "zh".to_string(),
            hide_completed_tasks: false,
            launch_at_startup: false,
            window_width: None,
            window_height: None,
            window_x: None,
            window_y: None,
            font_size: 14,
            font_family: String::new(),
            clipboard_view_mode: "normal".to_string(),
            clipboard_stack_gap: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardCategory {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
    pub created_at: i64,
}

impl ClipboardCategory {
    pub fn new(name: String, color: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            sort_order: 0,
            created_at: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub category_id: String,
    pub title: String,
    pub content: String,
    pub image_base64: Option<String>,
    pub image_path: Option<String>,
    pub thumbnail_base64: Option<String>,
    pub priority: i32,
    pub sort_order: i32,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub source: String,
    pub categories: Vec<Category>,
    pub tasks: Vec<Task>,
    pub clipboard_categories: Vec<ClipboardCategory>,
    pub clipboard_items: Vec<ClipboardItem>,
    pub settings: AppSettings,
}