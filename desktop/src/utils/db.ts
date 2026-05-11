import { invoke } from '@tauri-apps/api/core';
import type { Category, Task, AppSettings, ExportData, ClipboardCategory, ClipboardItem, BackupSettings, BackupInfo } from '../types';

// 分类操作
export async function getCategories(): Promise<Category[]> {
  return invoke('get_categories');
}

export async function addCategory(name: string, color: string): Promise<Category> {
  return invoke('add_category', { name, color });
}

export async function updateCategory(category: Category): Promise<void> {
  return invoke('update_category', { category });
}

export async function deleteCategory(id: string): Promise<void> {
  return invoke('delete_category', { id });
}

export async function ensureDefaultCategory(): Promise<string> {
  return invoke('ensure_default_category');
}

// 任务操作
export async function getTasks(): Promise<Task[]> {
  return invoke('get_tasks');
}

export async function addTask(task: Omit<Task, 'id' | 'createdAt' | 'updatedAt'>): Promise<Task> {
  return invoke('add_task', { task });
}

export async function updateTask(task: Task): Promise<void> {
  return invoke('update_task', { task });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke('delete_task', { id });
}

export async function reorderTasks(ids: string[]): Promise<void> {
  return invoke('reorder_tasks', { ids });
}

export async function reorderCategories(ids: string[]): Promise<void> {
  return invoke('reorder_categories', { ids });
}

export async function resetTaskSort(): Promise<void> {
  return invoke('reset_task_sort');
}

// 设置操作
export async function getSettings(): Promise<AppSettings> {
  return invoke('get_settings');
}

export async function updateSettings(settings: AppSettings): Promise<void> {
  return invoke('update_settings', { settings });
}

// 数据导出导入
export async function exportData(): Promise<ExportData> {
  return invoke('export_data');
}

export async function importData(data: ExportData): Promise<void> {
  return invoke('import_data', { data });
}

// 剪贴板分类操作
export async function getClipboardCategories(): Promise<ClipboardCategory[]> {
  return invoke('get_clipboard_categories');
}

export async function addClipboardCategory(name: string, color: string): Promise<ClipboardCategory> {
  return invoke('add_clipboard_category', { name, color });
}

export async function updateClipboardCategory(category: ClipboardCategory): Promise<void> {
  return invoke('update_clipboard_category', { category });
}

export async function deleteClipboardCategory(id: string): Promise<void> {
  return invoke('delete_clipboard_category', { id });
}

export async function reorderClipboardCategories(ids: string[]): Promise<void> {
  return invoke('reorder_clipboard_categories', { ids });
}

// 剪贴板项目操作
export async function getClipboardItems(): Promise<ClipboardItem[]> {
  return invoke('get_clipboard_items');
}

export async function addClipboardItem(item: Omit<ClipboardItem, 'id' | 'createdAt'>): Promise<ClipboardItem> {
  return invoke('add_clipboard_item', { item });
}

export async function updateClipboardItem(item: ClipboardItem): Promise<void> {
  return invoke('update_clipboard_item', { item });
}

export async function deleteClipboardItem(id: string): Promise<void> {
  return invoke('delete_clipboard_item', { id });
}

export async function reorderClipboardItems(ids: string[]): Promise<void> {
  return invoke('reorder_clipboard_items', { ids });
}

export async function readClipboardImageFile(path: string): Promise<string> {
  return invoke('read_clipboard_image_file', { path });
}

export async function setClipboardItemExpiry(id: string, expiresAt: number | null): Promise<void> {
  return invoke('set_clipboard_item_expiry', { id, expiresAt });
}

export async function cleanupExpiredItems(): Promise<number> {
  return invoke('cleanup_expired_items');
}

// 备份操作
export async function getBackupSettings(): Promise<BackupSettings> {
  return invoke('get_backup_settings');
}

export async function updateBackupSettings(settings: BackupSettings): Promise<void> {
  return invoke('update_backup_settings', { settings });
}

export async function createBackupNow(): Promise<string> {
  return invoke('create_backup_now');
}

export async function listBackups(): Promise<BackupInfo[]> {
  return invoke('list_backups');
}

export async function restoreBackup(filename: string): Promise<void> {
  return invoke('restore_backup', { filename });
}

export async function deleteBackup(filename: string): Promise<void> {
  return invoke('delete_backup', { filename });
}