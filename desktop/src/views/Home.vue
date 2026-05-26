<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import {
  NButton, NIcon, NInput, NDropdown,
  useMessage, useDialog
} from 'naive-ui';
import {
  AddOutline as AddIcon, SettingsOutline as SettingsIcon,
  SunnyOutline as LightIcon, MoonOutline as DarkIcon,
  CloseOutline as CloseIcon, RemoveOutline as MinusIcon,
  ExpandOutline as MaximizeIcon, ContractOutline as RestoreIcon,
  FlashOutline as CompactIcon,
  ListOutline as ListIcon, ClipboardOutline as ClipboardIcon,
  CopyOutline as CopyIcon, LayersOutline as StackedIcon,
  SwapVerticalOutline as SortIcon,
  CreateOutline as EditIcon, TrashOutline as DeleteIcon, LockClosedOutline as LockIcon
} from '@vicons/ionicons5';
import { h } from 'vue';
import draggable from 'vuedraggable';
import { getCurrentWindow, LogicalSize, LogicalPosition } from '@tauri-apps/api/window';
import { availableMonitors } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { useCategoryStore } from '../stores/categoryStore';
import { useTaskStore } from '../stores/taskStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useClipboardStore } from '../stores/clipboardStore';
import { useI18n } from '../composables/useI18n';
import CategoryTabs from '../components/CategoryTabs.vue';
import ClipboardCategoryTabs from '../components/ClipboardCategoryTabs.vue';
import ClipboardPanel from '../components/ClipboardPanel.vue';
import SettingsPage from '../components/SettingsPage.vue';
import SearchBar from '../components/SearchBar.vue';
import TaskCard from '../components/TaskCard.vue';
import TaskFormModal from '../components/TaskFormModal.vue';
import type { Task, ClipboardCategory } from '../types';
import { isBuiltinClipboardCategory } from '../types';

// 检测是否为 Windows
const isWindows = navigator.userAgent.toLowerCase().includes('windows');

// 检测是否为 Mac
const isMac = /mac/i.test(navigator.userAgent);

// 窗口最大化状态
const isMaximized = ref(false);
// 最大化前的窗口尺寸
const previousSize = ref<{ width: number; height: number } | null>(null);

async function checkMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch (_) {}
}

// 切换最大化/还原
async function toggleMaximize() {
  try {
    if (isMaximized.value) {
      // 还原到之前的尺寸
      if (previousSize.value) {
        await appWindow.setSize(new LogicalSize(previousSize.value.width, previousSize.value.height));
      } else {
        // 如果没有保存的尺寸，使用数据库中的尺寸或默认尺寸
        const { windowWidth, windowHeight } = settingsStore.settings;
        const width = windowWidth || 680;
        const height = windowHeight || 600;
        await appWindow.setSize(new LogicalSize(width, height));
      }
      isMaximized.value = false;
    } else {
      // 保存当前尺寸
      const size = await appWindow.innerSize();
      previousSize.value = { width: size.width, height: size.height };
      // 同时保存到 settings（用于下次启动还原）
      settingsStore.setWindowSize(size.width, size.height);
      // 最大化
      await appWindow.maximize();
      isMaximized.value = true;
    }
  } catch (e) {
    console.error('切换最大化失败:', e);
  }
}

const categoryStore = useCategoryStore();
const taskStore = useTaskStore();
const settingsStore = useSettingsStore();
const clipboardStore = useClipboardStore();
const appWindow = getCurrentWindow();
const message = useMessage();
const dialog = useDialog();
const { t } = useI18n();

// 剪贴板分类右键菜单
const clipCatContextMenuShow = ref(false);
const clipCatContextMenuX = ref(0);
const clipCatContextMenuY = ref(0);
const clipCatContextCat = ref<ClipboardCategory | null>(null);

const clipCatContextMenuOptions = computed(() => {
  if (!clipCatContextCat.value) return [];
  const cat = clipCatContextCat.value;
  const isBuiltin = isBuiltinClipboardCategory(cat.id);
  const options: any[] = [
    { label: '编辑名称', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
    { label: '设置颜色', key: 'color' },
    { label: cat.locked ? '解锁分类' : '锁定分类', key: 'lock', icon: () => h(NIcon, { component: LockIcon, size: 14, style: { color: cat.locked ? '#E05252' : '#333' } }) },
  ];
  if (!isBuiltin) {
    options.push({ type: 'divider', key: 'd1' });
    options.push({ label: '删除分类', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) });
  }
  return options;
});

function handleClipCatContextMenu(e: MouseEvent, cat: ClipboardCategory) {
  e.preventDefault();
  clipCatContextCat.value = cat;
  clipCatContextMenuX.value = e.clientX;
  clipCatContextMenuY.value = e.clientY;
  clipCatContextMenuShow.value = true;
}

async function handleClipCatMenuSelect(key: string) {
  clipCatContextMenuShow.value = false;
  if (!clipCatContextCat.value) return;
  const cat = clipCatContextCat.value;

  if (key === 'edit') {
    // 编辑名称：切换到正常模式进行编辑
    isCompactMode.value = false;
    // 选中该分类
    clipboardStore.selectCategory(cat.id);
    message.info('请在正常模式下编辑分类名称');
  } else if (key === 'color') {
    // 设置颜色：切换到正常模式
    isCompactMode.value = false;
    clipboardStore.selectCategory(cat.id);
    message.info('请在正常模式下设置分类颜色');
  } else if (key === 'lock') {
    const result = await clipboardStore.toggleCategoryLock(cat);
    message.success(result === 'locked' ? '分类已锁定，该分类下所有卡片禁止删除' : '分类已解锁');
  } else if (key === 'delete') {
    deleteClipboardCategory(cat);
  }
}

function deleteClipboardCategory(cat: ClipboardCategory) {
  dialog.warning({
    title: '确认删除',
    content: `确定删除分类"${cat.name}"及其所有剪贴板项目？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await clipboardStore.removeCategory(cat.id);
      message.success('删除成功');
    }
  });
}

// 窗口置顶状态
const isAlwaysOnTop = ref(false);

// 精简模式状态
const isCompactMode = ref(false);

// 精简前的窗口尺寸（用于恢复）
const preCompactSize = ref<{ width: number; height: number } | null>(null);

// 精简模式下窗口尺寸
const COMPACT_WIDTH = 400;

// 面板切换
const activePanel = ref<'tasks' | 'clipboard'>('tasks');

async function switchPanel(panel: 'tasks' | 'clipboard') {
  // 如果在设置页面，先退出设置页面
  if (currentPage.value === 'settings') {
    currentPage.value = 'main';
  }
  activePanel.value = panel;
  if (panel === 'clipboard') {
    // 先清理失效的图片项
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cleanup_invalid_image_items');
    } catch (e) {
      console.error('清理失效图片失败:', e);
    }
    await clipboardStore.load();
  }
}

// 剪贴板搜索
const clipboardSearchQuery = ref('');

function onClipboardSearch(val: string) {
  clipboardStore.searchQuery = val;
}

// 从剪贴板粘贴
async function handlePasteClipboard() {
  await clipboardStore.pasteFromClipboard({
    success: (msg: string) => message.success(msg),
    warning: (msg: string) => message.warning(msg),
  });
}

// 切换窗口置顶（不影响界面样式）
async function toggleAlwaysOnTop() {
  try {
    isAlwaysOnTop.value = !isAlwaysOnTop.value;
    await appWindow.setAlwaysOnTop(isAlwaysOnTop.value);
  } catch (e) {
    console.error('置顶失败:', e);
  }
}

// 切换精简模式（控制窗口尺寸和界面样式）
async function toggleCompactMode() {
  try {
    isCompactMode.value = !isCompactMode.value;

    if (isCompactMode.value) {
      // 进入精简模式：缩小宽度，高度继承当前高度
      const size = await appWindow.innerSize();
      preCompactSize.value = { width: size.width, height: size.height };
      // 精简模式：宽度固定 400，高度继承之前的高度
      await appWindow.setSize(new LogicalSize(COMPACT_WIDTH, size.height));
    } else {
      // 退出精简模式：恢复之前尺寸
      if (preCompactSize.value) {
        await appWindow.setSize(new LogicalSize(preCompactSize.value.width, preCompactSize.value.height));
      }
    }
  } catch (e) {
    console.error('切换精简模式失败:', e);
  }
}

// 页面切换
const currentPage = ref<'main' | 'settings'>('main');

// 任务列表 ref
const taskListRef = ref<HTMLElement | null>(null);

// 弹窗状态
const showTaskForm = ref(false);
const editingTask = ref<Task | null>(null);

// 拖拽状态
const isDragging = ref(false);
// 搜索时或自动排序时禁用拖拽
const dragEnabled = computed(() => !taskStore.searchQuery && taskStore.sortMode === 'custom');

const filteredTasks = computed(() => {
  let tasks = [...taskStore.tasks];

  // 按分类过滤
  if (categoryStore.selectedCategoryId) {
    tasks = tasks.filter(t => t.categoryId === categoryStore.selectedCategoryId);
  }

  // 搜索过滤
  if (taskStore.searchQuery) {
    const q = taskStore.searchQuery.toLowerCase();
    tasks = tasks.filter(t =>
      t.title.toLowerCase().includes(q) ||
      (t.description?.toLowerCase().includes(q) ?? false)
    );
  }

  // 隐藏已完成
  if (settingsStore.settings.hideCompletedTasks) {
    tasks = tasks.filter(t => t.status !== 'done');
  }

  // 根据排序模式排序
  const sortMode = taskStore.sortMode;
  if (sortMode === 'name') {
    // 按名字升序（A→Z）
    tasks.sort((a, b) => a.title.localeCompare(b.title, 'zh'));
  } else if (sortMode === 'updatedAt') {
    // 按修改日期降序（最新在前）
    tasks.sort((a, b) => b.updatedAt - a.updatedAt);
  } else {
    // 自定义排序：按 sortOrder
    tasks.sort((a, b) => a.sortOrder - b.sortOrder);
  }

  return tasks;
});

// 拖拽用列表：独立的 ref
const taskList = ref<Task[]>([]);

// 同步 filteredTasks 到 taskList
watch(filteredTasks, (val) => {
  taskList.value = [...val];
}, { immediate: true });

// 拖拽事件处理
function onDragStart() {
  isDragging.value = true;
  console.log('拖拽开始');
}

function onDragEnd() {
  isDragging.value = false;
  console.log('拖拽结束，新顺序:', taskList.value.map(t => t.title));
  const ids = taskList.value.map(t => t.id);
  taskStore.reorder(ids);
}

// 初始化加载
onMounted(async () => {
  await Promise.all([
    categoryStore.load(),
    taskStore.load(),
    settingsStore.load(),
    clipboardStore.load(),
  ]);

  // 窗口尺寸迁移：旧用户窗口太窄/太矮时重置
  const { windowWidth, windowHeight } = settingsStore.settings;
  const minWidth = 400;
  const minHeight = 600;
  if (windowWidth && windowWidth < minWidth) {
    settingsStore.setWindowSize(minWidth, windowHeight || minHeight);
    await appWindow.setSize(new LogicalSize(minWidth, windowHeight || minHeight));
  }
  if (windowHeight && windowHeight < minHeight) {
    settingsStore.setWindowSize(windowWidth || minWidth, minHeight);
    await appWindow.setSize(new LogicalSize(windowWidth || minWidth, minHeight));
  }

  // 应用保存的窗口尺寸和位置
  applyWindowState();

  // 检查窗口最大化状态
  checkMaximized();

  // 监听窗口尺寸和位置变化
  await setupResizeListener();
  await setupMoveListener();
});

onUnmounted(() => {
  removeResizeListener();
  removeMoveListener();
});

// 应用保存的窗口尺寸和位置
async function applyWindowState() {
  const { windowWidth, windowHeight, windowX, windowY } = settingsStore.settings;
  if (windowWidth && windowHeight) {
    try {
      await appWindow.setSize(new LogicalSize(windowWidth, windowHeight));
    } catch (_) {}
  }
  if (windowX && windowY) {
    try {
      await appWindow.setPosition(new LogicalPosition(windowX, windowY));
    } catch (_) {}
  }
}

// 窗口尺寸变化监听（带防抖）
let resizeTimeout: ReturnType<typeof setTimeout> | null = null;
let resizeUnlisten: (() => void) | null = null;

async function handleResize() {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout);
  }
  resizeTimeout = setTimeout(async () => {
    try {
      // 先检查最大化状态，最大化时不保存尺寸（避免下次启动以最大化尺寸打开普通窗口）
      const maximized = await appWindow.isMaximized();
      if (!maximized) {
        const size = await appWindow.innerSize();
        settingsStore.setWindowSize(size.width, size.height);
      }
      // 更新最大化状态
      checkMaximized();
    } catch (_) {}
  }, 500);
}

async function setupResizeListener() {
  try {
    resizeUnlisten = await appWindow.onResized(handleResize);
  } catch (_) {}
}

function removeResizeListener() {
  if (resizeTimeout) {
    clearTimeout(resizeTimeout);
  }
  if (resizeUnlisten) {
    resizeUnlisten();
  }
}

// 窗口位置变化监听（带防抖）
let moveTimeout: ReturnType<typeof setTimeout> | null = null;
let moveUnlisten: (() => void) | null = null;

// 吸附阈值（px）
const SNAP_THRESHOLD = 30;

// 窗口边缘吸附
async function snapToEdge() {
  try {
    const pos = await appWindow.outerPosition();
    const innerSize = await appWindow.innerSize();
    const scaleFactor = await appWindow.scaleFactor();
    const outerWidth = innerSize.width / scaleFactor;

    // 获取主显示器尺寸
    const monitors = await availableMonitors();
    if (monitors.length === 0) return;
    const primary = monitors[0];
    const screenW = primary.size.width;

    let newX = pos.x;
    let newY = pos.y;
    let needSnap = false;

    // 左边缘吸附
    if (Math.abs(pos.x) < SNAP_THRESHOLD) {
      newX = 0;
      needSnap = true;
    }
    // 右边缘吸附
    if (Math.abs(pos.x + outerWidth - screenW) < SNAP_THRESHOLD) {
      newX = screenW - outerWidth;
      needSnap = true;
    }
    // 上边缘吸附
    if (Math.abs(pos.y) < SNAP_THRESHOLD) {
      newY = 0;
      needSnap = true;
    }

    if (needSnap) {
      await appWindow.setPosition(new LogicalPosition(newX, newY));
    }
  } catch (_) {}
}

async function handleMove() {
  if (moveTimeout) {
    clearTimeout(moveTimeout);
  }
  moveTimeout = setTimeout(async () => {
    try {
      const position = await appWindow.outerPosition();
      settingsStore.setWindowPosition(position.x, position.y);
    } catch (_) {}
    // 移动结束后检测边缘吸附
    await snapToEdge();
  }, 500);
}

async function setupMoveListener() {
  try {
    moveUnlisten = await appWindow.onMoved(handleMove);
  } catch (_) {}
}

function removeMoveListener() {
  if (moveTimeout) {
    clearTimeout(moveTimeout);
  }
  if (moveUnlisten) {
    moveUnlisten();
  }
}

const isDark = computed(() => {
  if (settingsStore.settings.themeMode === 'dark') return true;
  if (settingsStore.settings.themeMode === 'light') return false;
  // system
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
});

// 同步字体设置到 CSS 变量
function applyFontSettings() {
  const { fontSize, fontFamily } = settingsStore.settings;
  document.documentElement.style.setProperty('--task-font-size', fontSize + 'px');
  if (fontFamily) {
    document.documentElement.style.setProperty('--task-font-family', fontFamily);
  } else {
    document.documentElement.style.removeProperty('--task-font-family');
  }
}

// 监听字体设置变化，实时更新
watch(() => [settingsStore.settings.fontSize, settingsStore.settings.fontFamily], applyFontSettings, { immediate: true });

function toggleTheme() {
  const next = isDark.value ? 'light' : 'dark';
  settingsStore.setTheme(next);
}

// 切换语言
function toggleLanguage() {
  const next = settingsStore.settings.language === 'zh' ? 'en' : 'zh';
  settingsStore.setLanguage(next);
}

// 任务排序
const sortModeOptions = computed(() => [
  { label: t('sort.custom'), key: 'custom' },
  { label: t('sort.name'), key: 'name' },
  { label: t('sort.updatedAt'), key: 'updatedAt' },
]);

const sortModeLabel = computed(() => {
  const mode = taskStore.sortMode;
  if (mode === 'name') return t('sort.name');
  if (mode === 'updatedAt') return t('sort.updatedAt');
  return t('sort.custom');
});

function handleSortSelect(key: 'custom' | 'name' | 'updatedAt') {
  taskStore.setSortMode(key);
}

// 剪贴板排序
const clipboardSortModeOptions = computed(() => [
  { label: t('sort.custom'), key: 'custom' },
  { label: t('sort.name'), key: 'name' },
  { label: t('sort.createdAt'), key: 'createdAt' },
]);

const clipboardSortModeLabel = computed(() => {
  const mode = clipboardStore.sortMode;
  if (mode === 'name') return t('sort.name');
  if (mode === 'createdAt') return t('sort.createdAt');
  return t('sort.custom');
});

function handleClipboardSortSelect(key: 'custom' | 'name' | 'createdAt') {
  clipboardStore.setSortMode(key);
}

// 剪贴板视图切换
const isClipboardStacked = computed(() => settingsStore.settings.clipboardViewMode === 'stacked');

function toggleClipboardView() {
  settingsStore.setClipboardViewMode(isClipboardStacked.value ? 'normal' : 'stacked');
}

// 任务视图切换
const taskViewMode = computed(() => settingsStore.settings.taskViewMode);

function toggleTaskView() {
  settingsStore.setTaskViewMode(taskViewMode.value === 'normal' ? 'stacked' : 'normal');
}

// 计算层叠样式（响应式更新）
const taskStackStyle = computed(() => {
  if (taskViewMode.value === 'stacked') {
    const gap = settingsStore.settings.clipboardStackGap ?? 64;
    return { '--stack-gap': `${gap}px` };
  }
  return {};
});

// 打开任务表单（改为快速添加空白任务）
async function openAddTask() {
  await categoryStore.ensureDefaultCategory();
  const categoryId = categoryStore.selectedCategoryId || categoryStore.categories[0]?.id;
  if (categoryId) {
    const newTask = await taskStore.addQuickTask(categoryId);
    // 标记为编辑状态（通过 ID 传递）
    editingTaskId.value = newTask.id;
  }
}

// 当前正在编辑标题的任务 ID
const editingTaskId = ref<string | null>(null);

function onTaskSaved() {
  taskStore.load();
}

// 获取任务的分类颜色
function getCategoryColor(task: Task): string {
  const category = categoryStore.categories.find(c => c.id === task.categoryId);
  return category?.color || '';
}

// 编辑任务
function editTask(task: Task) {
  editingTask.value = task;
  showTaskForm.value = true;
}

// 删除任务
async function deleteTask(id: string) {
  const task = taskStore.tasks.find(t => t.id === id);
  if (task && taskStore.isTaskLocked(task)) {
    message.warning('锁定的分类下任务不能被删除');
    return;
  }
  await taskStore.remove(id);
}

// 切换任务状态
function toggleTaskStatus(task: Task) {
  taskStore.toggleStatus(task);
}

// 更新任务优先级
function updateTaskPriority(task: Task, priority: 1 | 2 | 3) {
  task.priority = priority;
  taskStore.update(task);
}

// 更新任务分类
function updateTaskCategory(task: Task, categoryId: string) {
  task.categoryId = categoryId;
  taskStore.update(task);
}

// 更新开始日期
function updateTaskStartDate(task: Task, startDate: number | undefined) {
  task.startDate = startDate;
  taskStore.update(task);
}

// 更新截止日期
function updateTaskDueDate(task: Task, dueDate: number | undefined) {
  task.dueDate = dueDate;
  taskStore.update(task);
}

// 更新任务标题
function updateTaskTitle(task: Task, title: string) {
  task.title = title;
  taskStore.update(task);
  editingTaskId.value = null; // 结束编辑状态
}

// 更新任务描述
function updateTaskDescription(task: Task, description: string | undefined) {
  task.description = description;
  taskStore.update(task);
}

// 更新任务缩略图
function updateTaskThumbnail(task: Task, thumbnailBase64: string | undefined) {
  task.thumbnailBase64 = thumbnailBase64;
  taskStore.update(task);
}

// 移动任务到最顶部
function moveTaskToTop(task: Task) {
  const categoryTasks = taskStore.tasks.filter((t: Task) => t.categoryId === task.categoryId);
  const minSort = Math.min(...categoryTasks.map((t: Task) => t.sortOrder));
  task.sortOrder = (minSort || 0) - 1;
  taskStore.update(task);
}

// 任务列表区域右键菜单
const taskListContextMenuShow = ref(false);
const taskListContextMenuX = ref(0);
const taskListContextMenuY = ref(0);

function handleTaskListContextMenu(e: MouseEvent) {
  // 只在空白区域（非任务卡片）触发右键菜单
  const target = e.target as HTMLElement;
  if (target.closest('.task-wrapper, .simple-card')) return;

  e.preventDefault();
  taskListContextMenuX.value = e.clientX;
  taskListContextMenuY.value = e.clientY;
  taskListContextMenuShow.value = true;
}

function handleTaskListMenuSelect(key: string) {
  taskListContextMenuShow.value = false;
  if (key === 'addTask') {
    openAddTask();
  }
}

// 打开设置
function openSettingsPage() {
  currentPage.value = 'settings';
}

// 返回主页
function goBackToMain() {
  currentPage.value = 'main';
}

/// 全局窗口拖拽（header 区域，排除交互元素）
async function startWindowDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest('button, .mac-window-controls, .win-controls')) {
    return;
  }
  try {
    await appWindow.startDragging();
  } catch (_) {}
}

/// 分类标签区域拖拽（点击空白处拖拽窗口）
async function startTabsDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 只有点击在标签按钮上才不拖拽
  if (target.closest('.tab-btn')) {
    return;
  }
  try {
    await appWindow.startDragging();
  } catch (_) {}
}

/// 隐藏窗口到系统托盘（隐藏前保存窗口尺寸和位置）
async function hideToTray() {
  try {
    // 隐藏前保存窗口尺寸和位置（避免防抖延迟导致最后尺寸丢失）
    const maximized = await appWindow.isMaximized();
    if (!maximized) {
      const size = await appWindow.innerSize();
      const position = await appWindow.outerPosition();
      settingsStore.setWindowSize(size.width, size.height);
      settingsStore.setWindowPosition(position.x, position.y);
    }
    await invoke('hide_to_tray');
  } catch (_) {}
}

</script>

<template>
  <div class="app-layout">
    <!-- Windows 端：全局 Header -->
    <div v-if="isWindows" class="global-header" @mousedown="startWindowDrag">
      <div class="header">
        <div class="window-controls win-controls">
          <NButton quaternary size="tiny" class="win-btn" @click="appWindow.minimize()">
            <template #icon>
              <NIcon :component="MinusIcon" :size="12" />
            </template>
          </NButton>
          <NButton quaternary size="tiny" class="win-btn" @click="toggleMaximize()">
            <template #icon>
              <NIcon :component="isMaximized ? RestoreIcon : MaximizeIcon" :size="12" />
            </template>
          </NButton>
          <NButton quaternary size="tiny" class="win-btn close-btn" @click="hideToTray()">
            <template #icon>
              <NIcon :component="CloseIcon" :size="12" />
            </template>
          </NButton>
        </div>
      </div>
    </div>

    <!-- 精简模式下剪贴板操作栏 -->
    <div v-if="isCompactMode && activePanel === 'clipboard'" class="compact-clipboard-actions">
      <button class="view-toggle-btn compact" @click="toggleClipboardView" :title="isClipboardStacked ? t('header.listView') : t('header.stackedView')">
        <NIcon :component="isClipboardStacked ? ListIcon : StackedIcon" size="16" />
      </button>
    </div>

    <!-- 侧边栏 + 内容区 -->
    <div class="body-area">
      <!-- 侧边栏 -->
      <nav class="sidebar" @mousedown="startWindowDrag">
        <!-- Mac 端：红黄绿按钮 -->
        <div v-if="isMac" class="mac-window-controls">
          <button class="mac-btn close" @click.stop="hideToTray()" title="关闭">
            <span class="mac-btn-icon">×</span>
          </button>
          <button class="mac-btn minimize" @click.stop="appWindow.minimize()" title="最小化">
            <span class="mac-btn-icon">−</span>
          </button>
          <button class="mac-btn maximize" @click.stop="toggleMaximize()" :title="isMaximized ? '还原' : '最大化'">
            <span class="mac-btn-icon">⧉</span>
          </button>
        </div>

        <div class="sidebar-buttons">
          <!-- 置顶按钮（第一个位置） -->
                <button
                  :class="['sidebar-btn', { active: isAlwaysOnTop }]"
                  @click="toggleAlwaysOnTop"
                  :title="isAlwaysOnTop ? t('sidebar.unpin') : t('sidebar.pin')"
                >
                  📌
                </button>
                <button
                  :class="['sidebar-btn', { active: activePanel === 'tasks' && currentPage === 'main' }]"
                  @click="switchPanel('tasks')"
                  :title="t('sidebar.tasks')"
                >
                  <NIcon :component="ListIcon" size="22" />
                </button>
                <button
                  :class="['sidebar-btn', { active: activePanel === 'clipboard' && currentPage === 'main' }]"
                  @click="switchPanel('clipboard')"
                  :title="t('sidebar.clipboard')"
                >
                  <NIcon :component="ClipboardIcon" size="22" />
                </button>
                <div class="sidebar-spacer" />
                <!-- 精简按钮（底部位置，黑白模式前面） -->
                <button
                  :class="['sidebar-btn', { active: isCompactMode }]"
                  @click="toggleCompactMode"
                  :title="isCompactMode ? t('sidebar.uncompact') : t('sidebar.compact')"
                >
                  <NIcon :component="CompactIcon" size="22" />
                </button>
                <button
                  :class="['sidebar-btn', { active: !isDark }]"
                  @click="toggleTheme"
                  :title="t('sidebar.toggleTheme')"
                >
                  <NIcon :component="isDark ? LightIcon : DarkIcon" size="22" />
                </button>
                <button
                  :class="['sidebar-btn lang-btn', { active: settingsStore.settings.language === 'en' }]"
                  @click="toggleLanguage"
                  :title="t('sidebar.toggleLanguage')"
                >
                  {{ settingsStore.settings.language === 'zh' ? '中' : 'En' }}
                </button>
                <button
                  :class="['sidebar-btn', { active: currentPage === 'settings' }]"
                  @click="currentPage === 'settings' ? goBackToMain() : openSettingsPage()"
                  :title="t('sidebar.settings')"
                >
                  <NIcon :component="SettingsIcon" size="22" />
                </button>
              </div>
            </nav>

            <!-- 主内容区 -->
            <div class="main-content">
              <!-- 任务面板 -->
              <div v-show="activePanel === 'tasks' && currentPage === 'main'" class="panel tasks-panel" :class="{ 'compact-panel': isCompactMode }">
                <!-- 非精简模式：分类 tabs -->
                <div v-if="!isCompactMode" class="panel-tabs" @mousedown="startTabsDrag">
                  <CategoryTabs />
                </div>

                <!-- 非精简模式：搜索行 -->
                <div v-if="!isCompactMode" class="panel-search-row">
                  <div class="task-search-row">
                    <NButton
                      type="primary" size="small"
                      @click="openAddTask"
                      class="search-action-btn"
                    >
                      <template #icon><NIcon :component="AddIcon" /></template>
                      {{ t('header.task') }}
                    </NButton>
                    <div class="search-bar-wrapper">
                      <SearchBar />
                    </div>
                    <button class="view-toggle-btn" @click="toggleTaskView" :title="taskViewMode === 'stacked' ? t('header.listView') : t('header.stackedView')">
                      <NIcon :component="taskViewMode === 'stacked' ? ListIcon : StackedIcon" size="16" />
                    </button>
                    <NDropdown
                      placement="bottom-end"
                      :options="sortModeOptions"
                      @select="handleSortSelect"
                    >
                      <button class="view-toggle-btn" :title="sortModeLabel">
                        <NIcon :component="SortIcon" size="16" />
                      </button>
                    </NDropdown>
                  </div>
                </div>

                <!-- 精简模式：剪贴板迷你分类切换器（任务面板不需要） -->

                <div class="task-list" ref="taskListRef" :class="{ 'compact-list': isCompactMode, 'stacked-list': taskViewMode === 'stacked' }" :style="taskStackStyle" @contextmenu="handleTaskListContextMenu">
                  <div v-if="filteredTasks.length === 0" class="empty">
                    {{ t('empty.noTasks') }}
                  </div>
                  <draggable
                    v-else
                    v-model="taskList"
                    :disabled="!dragEnabled"
                    item-key="id"
                    ghost-class="ghost"
                    chosen-class="chosen"
                    drag-class="dragging"
                    :animation="200"
                    :force-fallback="true"
                    :fallback-tolerance="3"
                    class="drag-container"
                    :class="{ 'is-dragging': isDragging }"
                    @start="onDragStart"
                    @end="onDragEnd"
                  >
                    <template #item="{ element }">
                      <div class="task-wrapper">
                        <TaskCard
                          :task="element"
                          :category-color="getCategoryColor(element)"
                          :categories="categoryStore.categories"
                          :is-editing-title="editingTaskId === element.id"
                          :compact="isCompactMode"
                          @edit="editTask"
                          @delete="deleteTask"
                          @toggle-status="toggleTaskStatus"
                          @update-priority="updateTaskPriority"
                          @update-category="updateTaskCategory"
                          @update-start-date="updateTaskStartDate"
                          @update-due-date="updateTaskDueDate"
                          @update-title="updateTaskTitle"
                          @update-description="updateTaskDescription"
                          @update-thumbnail="updateTaskThumbnail"
                          @move-to-top="moveTaskToTop"
                        />
                      </div>
                    </template>
                  </draggable>
                </div>

                <TaskFormModal
                  :show="showTaskForm"
                  :task="editingTask"
                  @close="showTaskForm = false"
                  @saved="onTaskSaved"
                />

                <!-- 任务列表右键菜单 -->
                <NDropdown
                  placement="bottom-start"
                  trigger="manual"
                  :x="taskListContextMenuX"
                  :y="taskListContextMenuY"
                  :show="taskListContextMenuShow"
                  :options="[
                    { label: t('contextMenu.addTask'), key: 'addTask', icon: () => h(NIcon, { component: AddIcon, size: 16 }) }
                  ]"
                  @select="handleTaskListMenuSelect"
                  @clickoutside="taskListContextMenuShow = false"
                />
              </div>

              <!-- 剪贴板面板 -->
              <div v-show="activePanel === 'clipboard' && currentPage === 'main'" class="panel clipboard-panel" :class="{ 'compact-panel': isCompactMode }">
                <!-- 非精简模式：分类 tabs -->
                <div v-if="!isCompactMode" class="panel-tabs" @mousedown="startTabsDrag">
                  <ClipboardCategoryTabs />
                </div>

                <!-- 非精简模式：搜索行 -->
                <div v-if="!isCompactMode" class="panel-search-row">
                  <div class="clipboard-search-row">
                    <NButton
                      type="primary" size="small"
                      @click="handlePasteClipboard"
                      class="search-action-btn"
                    >
                      <template #icon><NIcon :component="CopyIcon" /></template>
                      {{ t('header.paste') }}
                    </NButton>
                    <NInput
                      v-model:value="clipboardSearchQuery"
                      :placeholder="t('header.searchClipboard')"
                      clearable size="small"
                      class="clipboard-search-input"
                      @update:value="onClipboardSearch"
                      @clear="clipboardSearchQuery = ''"
                    />
                    <button class="view-toggle-btn" @click="toggleClipboardView" :title="isClipboardStacked ? t('header.listView') : t('header.stackedView')">
                      <NIcon :component="isClipboardStacked ? ListIcon : StackedIcon" size="16" />
                    </button>
                    <NDropdown
                      placement="bottom-end"
                      :options="clipboardSortModeOptions"
                      @select="handleClipboardSortSelect"
                    >
                      <button class="view-toggle-btn" :title="clipboardSortModeLabel">
                        <NIcon :component="SortIcon" size="16" />
                      </button>
                    </NDropdown>
                  </div>
                </div>

                <!-- 精简模式：剪贴板迷你分类切换器 - 与正常模式同步 -->
                <div v-if="isCompactMode" class="compact-clip-filter">
                  <!-- 全部 tab -->
                  <button
                    :class="['clip-tab', 'all-tab', { active: clipboardStore.selectedCategoryId === null }]"
                    @click="clipboardStore.selectCategory(null)"
                  >{{ t('compact.all') }}</button>

                  <!-- 内置分类 tab -->
                  <button
                    v-for="cat in clipboardStore.builtinCategories"
                    :key="cat.id"
                    :class="['clip-tab', 'builtin-tab', { active: clipboardStore.selectedCategoryId === cat.id }]"
                    :style="{ '--tab-color': cat.color }"
                    @click="clipboardStore.selectCategory(cat.id)"
                    @contextmenu="handleClipCatContextMenu($event, cat)"
                  >{{ cat.name === '文本' || cat.name === 'Text' ? t('compact.text') : t('compact.image') }}</button>

                  <!-- 自定义分类 tab -->
                  <button
                    v-for="cat in clipboardStore.customCategories"
                    :key="cat.id"
                    :class="['clip-tab', { active: clipboardStore.selectedCategoryId === cat.id }]"
                    :style="{ '--tab-color': cat.color }"
                    @click="clipboardStore.selectCategory(cat.id)"
                    @contextmenu="handleClipCatContextMenu($event, cat)"
                  >
                    <span v-if="cat.locked" style="margin-right: 2px;">🔒</span>
                    <span :style="{ color: cat.color }">{{ cat.name }}</span>
                  </button>
                </div>

                <!-- 剪贴板分类右键菜单 -->
                <NDropdown
                  placement="bottom-start"
                  trigger="manual"
                  :x="clipCatContextMenuX"
                  :y="clipCatContextMenuY"
                  :show="clipCatContextMenuShow"
                  :options="clipCatContextMenuOptions"
                  @select="handleClipCatMenuSelect"
                  @clickoutside="clipCatContextMenuShow = false"
                />

                <ClipboardPanel :compact="isCompactMode" :stacked="isClipboardStacked" :stack-gap="settingsStore.settings.clipboardStackGap" />
              </div>

              <!-- 设置页面 -->
              <SettingsPage v-if="currentPage === 'settings'" @back="goBackToMain" />
            </div>
          </div>
        </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: transparent;
  color: #333;
}

/* 深色模式 */
html.dark, html.dark body, html.dark #app {
  background: transparent;
  color: #e0e0e0;
}

.app-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 0;
  overflow: hidden;
  background: transparent;
}

.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: #f5f5f5;
  /* 使用 clip-path 强制裁剪角落，消除白色像素 */
  clip-path: inset(0 round 10px);
}

html.dark .app-layout {
  background: #1a1a1a;
}

/* 全局 Header（仅 Windows） */
.global-header {
  position: relative;
  flex-shrink: 0;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
  padding: 8px 12px;
  border-radius: 10px 10px 0 0;
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

html.dark .global-header {
  background: #1a1a1a;
  border-bottom-color: #333;
}

.global-header .header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

/* Mac 端自定义红黄绿按钮 */
.mac-window-controls {
  display: flex;
  gap: 8px;
  align-items: center;
}

.mac-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  padding: 0;
  transition: filter 0.1s;
}

.mac-btn .mac-btn-icon {
  display: none;
  font-size: 10px;
  line-height: 1;
  font-weight: 700;
  color: rgba(0, 0, 0, 0.5);
  position: relative;
  top: -1px;
}

.mac-window-controls:hover .mac-btn .mac-btn-icon {
  display: block;
}

.mac-btn.close {
  background: #FF5F57;
}
.mac-btn.minimize {
  background: #FFBD2E;
}
.mac-btn.maximize {
  background: #28C840;
}

html.dark .mac-btn.close {
  background: #FF5F57;
}
html.dark .mac-btn.minimize {
  background: #FFBD2E;
}
html.dark .mac-btn.maximize {
  background: #28C840;
}

html.dark .mac-btn .mac-btn-icon {
  color: rgba(0, 0, 0, 0.6);
}

/* 侧边栏 + 内容区 */
.body-area {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.sidebar {
  width: 68px;
  flex-shrink: 0;
  background: rgba(245, 245, 245, 0.95);
  display: flex;
  flex-direction: column;
  z-index: 10;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px 0 0 10px;
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

html.dark .sidebar {
  background: rgba(26, 26, 26, 0.95);
  border-right-color: rgba(255, 255, 255, 0.08);
}

/* Mac 端：侧边栏顶部红黄绿按钮区域 */
.mac-window-controls {
  flex-shrink: 0;
  display: flex;
  justify-content: center;
  gap: 8px;
  padding: 8px 0 6px;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.sidebar-buttons {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding-top: 4px;
  flex: 1;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.sidebar-btn {
  width: 40px;
  height: 40px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #666;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  flex-shrink: 0;
}

.sidebar-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #333;
}

html.dark .sidebar-btn {
  color: #888;
}

html.dark .sidebar-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ddd;
}

.sidebar-btn.active {
  background: rgba(74, 144, 217, 0.25);
  color: #4A90D9;
}

html.dark .sidebar-btn.active {
  background: rgba(74, 144, 217, 0.25);
  color: #4A90D9;
}

.sidebar-btn.lang-btn {
  font-size: 14px;
  font-weight: 600;
}

.sidebar-spacer {
  flex: 1;
}

.main-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 0 10px 10px 0;
}

.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
  padding: 0 12px;
  background: #f5f5f5;
}

html.dark .panel {
  background: #1a1a1a;
}

.panel.compact-panel {
  padding: 12px;
}

/* 面板内分类 tabs 区域 */
.panel-tabs {
  flex-shrink: 0;
  padding-top: 0;
  padding-bottom: 0;
  cursor: default;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

/* 面板内搜索行 */
.panel-search-row {
  flex-shrink: 0;
  padding: 4px 0 8px;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.tasks-panel .task-list,
.clipboard-panel .clipboard-list {
  padding-top: 12px;
  padding-bottom: 12px;
}

/* 精简模式下去掉 tabs 和搜索行的 padding */
.panel.compact-panel .panel-tabs,
.panel.compact-panel .panel-search-row {
  display: none;
}

html.dark .app-container {
  background: transparent;
}

/* 窗口拖拽区域 - Tauri v2 */
[data-tauri-drag-region] {
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

html.dark .header {
  background: #1a1a1a;
}

/* 固定顶部区域 */
.fixed-header {
  position: relative;
  flex-shrink: 0;
  background: #f5f5f5 !important;
  border-bottom: 1px solid #e0e0e0;
  margin: -12px -12px 0 -12px;
  padding: 12px 12px 0 12px;
}

html.dark .fixed-header {
  background: #1a1a1a !important;
  border-bottom-color: #333 !important;
}

.header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 8px;
  background: inherit;
}

html.dark .header {
  background: #1a1a1a;
}

/* 标题栏内的交互控件不可拖拽 */
.pin-control,
.window-controls,
.header .n-space {
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

/* 标签按钮本身可点击 */
.panel-tabs .tab-btn {
  cursor: pointer;
}

.pin-control {
  display: flex;
  align-items: center;
}

.pin-control .n-button {
  padding: 0 6px;
}

.pin-control .pin-emoji {
  font-size: 14px;
  line-height: 1;
  filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.3));
  transition: transform 0.2s ease;
  display: inline-block;
}

.pin-control .n-button:not(.n-button--primary-type) .pin-emoji {
  transform: rotate(45deg);
  opacity: 0.6;
}

.window-controls {
  display: flex;
  gap: 8px;
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  cursor: pointer;
}

.dot.close { background: #FF5F57; }
.dot.minimize { background: #FFBD2E; }
.dot.maximize { background: #28C840; }

/* Windows 按钮样式 */
.win-controls {
  gap: 4px;
}

.win-controls .win-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 6px;
}

.win-controls .win-btn:hover {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 6px;
}

html.dark .win-controls .win-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 6px;
}

.win-controls .close-btn:hover {
  background: #e81123 !important;
  color: #fff !important;
  border-radius: 6px;
}

/* 任务搜索行 */
.task-search-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.task-search-row .search-bar-wrapper {
  flex: 1;
  min-width: 0;
}

/* 剪贴板搜索行 */
.clipboard-search-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.clipboard-search-row .clipboard-search-input {
  flex: 1;
  min-width: 0;
}

.view-toggle-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 8px;
  background: rgba(100, 100, 100, 0.1);
  color: #888;
  cursor: pointer;
  transition: all 0.15s;
}

.view-toggle-btn:hover {
  background: rgba(100, 100, 100, 0.2);
  color: #4A90D9;
}

html.dark .view-toggle-btn {
  background: rgba(100, 100, 100, 0.2);
  color: #777;
}

html.dark .view-toggle-btn:hover {
  background: rgba(100, 100, 100, 0.3);
  color: #4A90D9;
}

.search-action-btn {
  flex-shrink: 0;
  margin-right: 8px;
}

/* 精简模式下剪贴板操作栏 — 紧贴右侧，最小化占用 */
.compact-clipboard-actions {
  position: absolute;
  top: 10px;
  right: 14px;
  z-index: 200;
}

.view-toggle-btn.compact {
  width: 28px;
  height: 28px;
}
.header-action-btn {
  flex-shrink: 0;
  height: 28px;
  font-size: 13px;
  padding: 0 12px;
}

/* 精简模式剪贴板分类 - 与正常模式 CategoryTabs 同步样式 */
.compact-clip-filter {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 0 8px;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  overflow-x: auto;
  scrollbar-width: none;
}

.compact-clip-filter::-webkit-scrollbar {
  display: none;
}

.clip-tab {
  padding: 4px 12px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #888;
  cursor: pointer;
  font-size: 14px;
  white-space: nowrap;
  transition: background 0.15s, color 0.15s;
  user-select: none;
}

.clip-tab:hover {
  background: rgba(74, 144, 217, 0.08);
  color: #4A90D9;
}

.clip-tab.active {
  color: var(--tab-color, #4A90D9);
  border-bottom: 2px solid var(--tab-color, #4A90D9);
  border-radius: 4px 4px 0 0;
}

.all-tab.active {
  background: rgba(74, 144, 217, 0.15);
  font-weight: 600;
  color: #4A90D9;
}

.builtin-tab.active {
  background: rgba(74, 144, 217, 0.12);
}

html.dark .clip-tab {
  color: #999;
}

html.dark .clip-tab:hover {
  background: rgba(74, 144, 217, 0.1);
  color: #5BA4F5;
}

html.dark .clip-tab.active {
  color: var(--tab-color, #5BA4F5);
  border-bottom-color: var(--tab-color, #5BA4F5);
}

html.dark .all-tab.active {
  background: rgba(74, 144, 217, 0.18);
  color: #5BA4F5;
}

html.dark .builtin-tab.active {
  background: rgba(74, 144, 217, 0.15);
}

/* 任务列表（唯一可滚动区域） */
.task-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px 6px;
}

/* 精简模式列表 */
.task-list.compact-list {
  padding: 0;
}

.task-list.compact-list .task-wrapper {
  margin-bottom: 6px;
}

/* 滚动条默认隐藏，hover/滚动时显示 */
.task-list::-webkit-scrollbar {
  width: 4px;
}

.task-list::-webkit-scrollbar-track {
  background: transparent;
}

.task-list::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 10px;
  transition: background 0.3s;
}

/* 鼠标移到列表区域时显示滚动条 - 浅色模式 */
.task-list:hover::-webkit-scrollbar-thumb {
  background: rgba(100, 100, 100, 0.2);
}

.task-list:hover::-webkit-scrollbar-thumb:hover {
  background: rgba(100, 100, 100, 0.35);
}

/* 深色模式滚动条 */
html.dark .task-list::-webkit-scrollbar-thumb {
  background: transparent;
}

html.dark .task-list:hover::-webkit-scrollbar-thumb {
  background: rgba(80, 80, 80, 0.4);
}

html.dark .task-list:hover::-webkit-scrollbar-thumb:hover {
  background: rgba(80, 80, 80, 0.6);
}

.drag-container {
  display: flex;
  flex-direction: column;
}

.task-wrapper {
  width: 100%;
  margin-bottom: 10px;
  user-select: none;
}

/* 任务层叠模式 */
.task-list.stacked-list .task-wrapper {
  margin-bottom: calc(-80px + var(--stack-gap, 64px));
  transition: transform 0.2s ease, z-index 0s;
}

.task-list.stacked-list .task-wrapper:last-child {
  margin-bottom: 0;
}

.task-list.stacked-list .task-wrapper:nth-child(odd) {
  transform: translateX(3px);
}

.task-list.stacked-list .task-wrapper:nth-child(even) {
  transform: translateX(-3px);
}

.task-list.stacked-list .task-wrapper:hover {
  z-index: 100;
  transform: translateY(-8px) scale(1.02);
}

/* 拖拽中：禁用卡片 hover 效果，避免干扰排挤动画 */
.drag-container.is-dragging .task-wrapper {
  cursor: grabbing;
}

.empty {
  text-align: center;
  padding: 40px;
  color: #888;
}

/* 固定底部区域 */
.footer {
  flex-shrink: 0;
  padding-top: 12px;
  background: #f5f5f5;
  border-top: 1px solid #e0e0e0;
  margin: 0 -12px -12px -12px;
  padding: 12px 12px 12px 12px;
}

html.dark .footer {
  background: #1a1a1a;
  border-top-color: #333;
}

/* 拖拽动画效果 */
.ghost {
  opacity: 0.2;
  border: 2px dashed #4A90D9;
  border-radius: 12px;
  background: rgba(74, 144, 217, 0.05);
}

html.dark .ghost {
  border-color: #5BA4F5;
  background: rgba(74, 144, 217, 0.08);
}

.chosen {
  opacity: 0.95;
  box-shadow: 0 8px 24px rgba(74, 144, 217, 0.2);
}

html.dark .chosen {
  box-shadow: 0 8px 24px rgba(74, 144, 217, 0.3);
}

.dragging {
  opacity: 1;
  box-shadow: 0 16px 40px rgba(74, 144, 217, 0.3);
  border-radius: 12px;
  transform: scale(1.03) rotate(1deg);
}

html.dark .dragging {
  box-shadow: 0 16px 40px rgba(74, 144, 217, 0.25);
}
</style>