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
}

export interface ExportData {
  version: string;
  exportedAt: string;
  source: 'desktop';
  categories: Category[];
  tasks: Task[];
  settings: AppSettings;
}

// 限制常量
export const FREE_CATEGORY_LIMIT = 9;
export const FREE_TASK_PER_CATEGORY_LIMIT = 50;