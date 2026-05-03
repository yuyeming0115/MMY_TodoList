import { invoke } from '@tauri-apps/api/core';
import type { Category, Task, AppSettings, ExportData } from '../types';

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