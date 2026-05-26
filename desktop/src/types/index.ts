// 数据类型定义 - 与小程序保持一致

export interface Category {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  createdAt: number;
  locked?: boolean; // 是否锁定（该分类及下属任务禁止删除）
}

export interface Task {
  id: string;
  categoryId: string;
  title: string;
  description?: string;
  startDate?: number;
  dueDate?: number;
  priority: 1 | 2 | 3;
  status: 'todo' | 'inProgress' | 'done';
  sortOrder: number;
  isPinned: boolean; // 置顶
  thumbnailBase64?: string;
  createdAt: number;
  updatedAt: number;
}

export interface AppSettings {
  themeMode: 'system' | 'light' | 'dark';
  language: 'zh' | 'en';
  hideCompletedTasks: boolean;
  launchAtStartup: boolean;
  windowWidth?: number;
  windowHeight?: number;
  windowX?: number;
  windowY?: number;
  fontSize: number;
  fontFamily: string;
  clipboardViewMode: 'normal' | 'stacked';
  clipboardStackGap: number; // 层叠模式卡片间距（px）
  taskViewMode: 'normal' | 'stacked'; // 任务视图模式
  globalShortcut?: string; // 全局快捷键（如 "Ctrl+Alt+D")
  taskSortMode?: 'custom' | 'name' | 'updatedAt'; // 任务排序模式
  customSortBackup?: Record<string, number>; // 切换排序时保存的 sortOrder 备份
  clipboardSortMode?: 'custom' | 'name' | 'createdAt'; // 剪贴板排序模式
  clipboardSortBackup?: Record<string, number>; // 剪贴板排序备份
}

export interface ExportData {
  version: string;
  exportedAt: string;
  source: 'desktop';
  categories: Category[];
  tasks: Task[];
  clipboardCategories: ClipboardCategory[];
  clipboardItems: ClipboardItem[];
  settings: AppSettings;
}

export interface ClipboardCategory {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  createdAt: number;
  locked?: boolean; // 是否锁定（该分类下所有卡片禁止删除）
}

export interface ClipboardItem {
  id: string;
  categoryId: string;
  title: string;
  content: string;
  imageBase64?: string;
  imagePath?: string;
  // thumbnailBase64 已移除：学习 Ditto，数据库只存路径，缩略图从文件动态生成
  priority: 1 | 2 | 3;
  sortOrder: number;
  createdAt: number;
  expiresAt?: number | null;
  locked?: boolean; // 是否锁定（禁止删除）
}

// 内置分类 ID 常量（不可删除）
export const BUILTIN_CLIPBOARD_CATEGORIES = {
  TEXT: 'builtin_text',
  IMAGE: 'builtin_image',
} as const;

export const BUILTIN_CLIPBOARD_CATEGORY_IDS = Object.values(BUILTIN_CLIPBOARD_CATEGORIES);

// 内置分类元数据
export const BUILTIN_CLIPBOARD_CATEGORY_META = [
  { id: BUILTIN_CLIPBOARD_CATEGORIES.TEXT, name: '文本', color: '#4A90D9', sortOrder: 0 },
  { id: BUILTIN_CLIPBOARD_CATEGORIES.IMAGE, name: '图像', color: '#28C840', sortOrder: 1 },
] as const;

export function isBuiltinClipboardCategory(id: string): boolean {
  return BUILTIN_CLIPBOARD_CATEGORY_IDS.includes(id as typeof BUILTIN_CLIPBOARD_CATEGORY_IDS[number]);
}

// 限制常量
export const FREE_CATEGORY_LIMIT = 9;
export const FREE_TASK_PER_CATEGORY_LIMIT = 50;

// 备份类型
export type BackupType = 'quick' | 'full';

// 备份设置
export interface BackupSettings {
  backupOnClose: boolean;
  backupHourly: boolean;
  retentionDays: number;
  defaultBackupType: BackupType; // 默认备份类型
}

// 备份信息
export interface BackupInfo {
  filename: string;
  createdAt: number;
  sizeBytes: number;
  backupType: BackupType; // 备份类型（从文件名推断）
}

// 备份预览信息
export interface BackupPreview {
  filename: string;
  createdAt: number;
  backupType: BackupType;
  categoriesCount: number;
  tasksCount: number;
  clipboardCategoriesCount: number;
  clipboardItemsCount: number;
  clipboardImageCount: number;
  hasSettings: boolean;
}

// 恢复选项
export interface RestoreOptions {
  overwrite: boolean; // 是否覆盖现有数据
  restoreTasks: boolean;
  restoreClipboard: boolean;
  restoreSettings: boolean;
}