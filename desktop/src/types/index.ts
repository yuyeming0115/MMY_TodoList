// 数据类型定义 - 与小程序保持一致

export interface Category {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  createdAt: number;
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
}

export interface ClipboardItem {
  id: string;
  categoryId: string;
  title: string;
  content: string;
  imageBase64?: string;
  imagePath?: string;
  thumbnailBase64?: string;
  priority: 1 | 2 | 3;
  sortOrder: number;
  createdAt: number;
  expiresAt?: number | null;
}

// 内置分类 ID 常量（不可删除）
export const BUILTIN_CLIPBOARD_CATEGORIES = {
  TEXT: 'builtin_text',
  IMAGE: 'builtin_image',
  FAVORITE: 'builtin_favorite',
} as const;

export const BUILTIN_CLIPBOARD_CATEGORY_IDS = Object.values(BUILTIN_CLIPBOARD_CATEGORIES);

// 内置分类元数据
export const BUILTIN_CLIPBOARD_CATEGORY_META = [
  { id: BUILTIN_CLIPBOARD_CATEGORIES.TEXT, name: '文本', color: '#4A90D9', sortOrder: 0 },
  { id: BUILTIN_CLIPBOARD_CATEGORIES.IMAGE, name: '图像', color: '#28C840', sortOrder: 1 },
  { id: BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE, name: '收藏', color: '#F39C12', sortOrder: 2 },
] as const;

export function isBuiltinClipboardCategory(id: string): boolean {
  return BUILTIN_CLIPBOARD_CATEGORY_IDS.includes(id as typeof BUILTIN_CLIPBOARD_CATEGORY_IDS[number]);
}

// 限制常量
export const FREE_CATEGORY_LIMIT = 9;
export const FREE_TASK_PER_CATEGORY_LIMIT = 50;

// 备份设置
export interface BackupSettings {
  backupDaily: boolean;
  backupOnClose: boolean;
  backupHourly: boolean;
  retentionDays: number;
}

// 备份信息
export interface BackupInfo {
  filename: string;
  createdAt: number;
  sizeBytes: number;
}