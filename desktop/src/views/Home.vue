<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import {
  NButton, NIcon, NInput, NDropdown,
  useMessage
} from 'naive-ui';
import {
  AddOutline as AddIcon, SettingsOutline as SettingsIcon,
  SunnyOutline as LightIcon, MoonOutline as DarkIcon,
  CloseOutline as CloseIcon, RemoveOutline as MinusIcon,
  ExpandOutline as MaximizeIcon, ContractOutline as RestoreIcon,
  ListOutline as ListIcon, ClipboardOutline as ClipboardIcon,
  CopyOutline as CopyIcon, LayersOutline as StackedIcon
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
import CategoryTabs from '../components/CategoryTabs.vue';
import ClipboardCategoryTabs from '../components/ClipboardCategoryTabs.vue';
import ClipboardPanel from '../components/ClipboardPanel.vue';
import SettingsPage from '../components/SettingsPage.vue';
import SearchBar from '../components/SearchBar.vue';
import TaskCard from '../components/TaskCard.vue';
import TaskFormModal from '../components/TaskFormModal.vue';
import type { Task } from '../types';

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
      }
      isMaximized.value = false;
    } else {
      // 保存当前尺寸
      const size = await appWindow.innerSize();
      previousSize.value = { width: size.width, height: size.height };
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

// 窗口置顶状态
const isPinned = ref(false);
// 置顶前的窗口尺寸
const prePinSize = ref<{ width: number; height: number } | null>(null);

// 精简模式下窗口尺寸
const COMPACT_WIDTH = 400;

// 面板切换
const activePanel = ref<'tasks' | 'clipboard'>('tasks');

// 精简模式下的剪贴板分类过滤
const compactClipFilter = ref<string | null>(null);

async function switchPanel(panel: 'tasks' | 'clipboard') {
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

// 切换窗口置顶 + 精简模式
async function togglePin() {
  try {
    isPinned.value = !isPinned.value;
    await appWindow.setAlwaysOnTop(isPinned.value);

    if (isPinned.value) {
      // 进入精简模式：保存当前尺寸，缩小窗口
      const size = await appWindow.innerSize();
      prePinSize.value = { width: size.width, height: size.height };
      await appWindow.setSize(new LogicalSize(COMPACT_WIDTH, 400));
    } else {
      // 退出精简模式：恢复之前尺寸
      if (prePinSize.value) {
        await appWindow.setSize(new LogicalSize(prePinSize.value.width, prePinSize.value.height));
      }
    }
  } catch (e) {
    console.error('置顶失败:', e);
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
// 搜索时禁用拖拽（其他情况均可拖拽）
const dragEnabled = computed(() => !taskStore.searchQuery);

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

  // 按 sortOrder 排序
  tasks.sort((a, b) => a.sortOrder - b.sortOrder);

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
  // 设置平台 class
  if (isMac) {
    document.documentElement.classList.add('platform-mac');
  }

  await Promise.all([
    categoryStore.load(),
    taskStore.load(),
    settingsStore.load(),
    clipboardStore.load(),
  ]);

  // 窗口宽度迁移：旧用户窗口太窄时重置
  const { windowWidth } = settingsStore.settings;
  if (windowWidth && windowWidth < 500) {
    settingsStore.setWindowSize(680, settingsStore.settings.windowHeight || 600);
    await appWindow.setSize(new LogicalSize(680, settingsStore.settings.windowHeight || 600));
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
      const size = await appWindow.innerSize();
      settingsStore.setWindowSize(size.width, size.height);
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

// 剪贴板视图切换
const isClipboardStacked = computed(() => settingsStore.settings.clipboardViewMode === 'stacked');

function toggleClipboardView() {
  settingsStore.setClipboardViewMode(isClipboardStacked.value ? 'normal' : 'stacked');
}

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

/// 全局窗口拖拽（仅用于固定区域，排除交互元素）
async function startWindowDrag(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 排除所有交互元素
  if (target.closest('button, a, input, select, textarea, [role="button"], .n-button, .n-input, .n-checkbox, .n-switch')) {
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

/// 隐藏窗口到系统托盘
async function hideToTray() {
  try {
    await invoke('hide_to_tray');
  } catch (_) {}
}

</script>

<template>
  <div class="app-layout">
          <!-- 全局 Header（最上方，全宽） -->
          <div class="global-header" @mousedown="startWindowDrag">
            <div class="header" data-tauri-drag-region>
              <!-- 置顶按钮（始终显示） -->
              <div class="pin-control">
                <NButton quaternary size="tiny" @click="togglePin" :type="isPinned ? 'primary' : 'default'" round>
                  <span class="pin-emoji">📌</span>
                </NButton>
              </div>

              <!-- 任务面板：分类 tabs -->
              <div v-if="activePanel === 'tasks' && !isPinned" class="tabs-wrapper" @mousedown="startTabsDrag">
                <CategoryTabs />
              </div>

              <!-- 剪贴板面板：分类 tabs -->
              <div v-else-if="activePanel === 'clipboard' && !isPinned" class="tabs-wrapper" @mousedown="startTabsDrag">
                <ClipboardCategoryTabs />
              </div>

              <!-- 精简模式：剪贴板迷你分类切换器 -->
              <div v-if="isPinned && activePanel === 'clipboard'" class="compact-clip-filter">
                <button
                  :class="['clip-pill', { active: compactClipFilter === null }]"
                  @click="compactClipFilter = null"
                >全部</button>
                <button
                  v-for="cat in clipboardStore.builtinCategories"
                  :key="cat.id"
                  :class="['clip-pill', { active: compactClipFilter === cat.id }]"
                  :style="{ '--pill-color': cat.color }"
                  @click="compactClipFilter = cat.id"
                >{{ cat.name === '文本' ? '文' : cat.name === '图像' ? '图' : '★' }}</button>
              </div>

              <!-- 任务面板：添加任务按钮 -->
              <NButton
                v-if="activePanel === 'tasks' && !isPinned"
                type="primary" size="tiny" round
                @click="openAddTask"
                class="header-action-btn"
              >
                <template #icon><NIcon :component="AddIcon" /></template>
                添加任务
              </NButton>

              <!-- 剪贴板面板：从剪贴板粘贴按钮 -->
              <NButton
                v-else-if="activePanel === 'clipboard' && !isPinned"
                type="primary" size="tiny" round
                @click="handlePasteClipboard"
                class="header-action-btn"
              >
                <template #icon><NIcon :component="CopyIcon" /></template>
                粘贴
              </NButton>

              <!-- 窗口控制按钮 -->
              <div class="window-controls win-controls" v-if="isWindows">
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

            <!-- 第二行：搜索栏 -->
            <div v-if="!isPinned" class="search-wrapper">
              <SearchBar v-if="activePanel === 'tasks'" />
              <div v-else-if="activePanel === 'clipboard'" class="clipboard-search-row">
                <NInput
                  v-model:value="clipboardSearchQuery"
                  placeholder="搜索剪贴板..."
                  clearable size="small"
                  class="clipboard-search-input"
                  @update:value="onClipboardSearch"
                  @clear="clipboardSearchQuery = ''"
                />
                <button class="view-toggle-btn" @click="toggleClipboardView" :title="isClipboardStacked ? '切换列表视图' : '切换层叠视图'">
                  <NIcon :component="isClipboardStacked ? ListIcon : StackedIcon" size="16" />
                </button>
              </div>
            </div>

            <!-- 精简模式下剪贴板视图切换按钮（紧贴右侧，最小化占用） -->
            <div v-if="isPinned && activePanel === 'clipboard'" class="compact-clipboard-actions">
              <button class="view-toggle-btn compact" @click="toggleClipboardView" :title="isClipboardStacked ? '切换列表视图' : '切换层叠视图'">
                <NIcon :component="isClipboardStacked ? ListIcon : StackedIcon" size="16" />
              </button>
            </div>
          </div>

          <!-- 侧边栏 + 内容区 -->
          <div class="body-area">
            <!-- 侧边栏 -->
            <nav class="sidebar">
              <div class="sidebar-buttons">
                <button
                  :class="['sidebar-btn', { active: activePanel === 'tasks' && currentPage === 'main' }]"
                  @click="switchPanel('tasks')"
                  title="任务"
                >
                  <NIcon :component="ListIcon" size="22" />
                </button>
                <button
                  :class="['sidebar-btn', { active: activePanel === 'clipboard' && currentPage === 'main' }]"
                  @click="switchPanel('clipboard')"
                  title="剪贴板"
                >
                  <NIcon :component="ClipboardIcon" size="22" />
                </button>
                <div class="sidebar-spacer" />
                <button
                  :class="['sidebar-btn', { active: !isDark }]"
                  @click="toggleTheme"
                  title="切换主题"
                >
                  <NIcon :component="isDark ? LightIcon : DarkIcon" size="22" />
                </button>
                <button
                  :class="['sidebar-btn', { active: currentPage === 'settings' }]"
                  @click="currentPage === 'settings' ? goBackToMain() : openSettingsPage()"
                  title="设置"
                >
                  <NIcon :component="SettingsIcon" size="22" />
                </button>
              </div>
            </nav>

            <!-- 主内容区 -->
            <div class="main-content">
              <!-- 任务面板 -->
              <div v-show="activePanel === 'tasks' && currentPage === 'main'" class="panel tasks-panel" :class="{ 'compact-panel': isPinned }">
                <div class="task-list" ref="taskListRef" :class="{ 'compact-list': isPinned }" @contextmenu="handleTaskListContextMenu">
                  <div v-if="filteredTasks.length === 0" class="empty">
                    暂无任务
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
                          :compact="isPinned"
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
                    { label: '添加任务', key: 'addTask', icon: () => h(NIcon, { component: AddIcon, size: 16 }) }
                  ]"
                  @select="handleTaskListMenuSelect"
                  @clickoutside="taskListContextMenuShow = false"
                />
              </div>

              <!-- 剪贴板面板 -->
              <div v-show="activePanel === 'clipboard' && currentPage === 'main'" class="panel clipboard-panel" :class="{ 'compact-panel': isPinned }">
                <ClipboardPanel :compact="isPinned" :category-filter="compactClipFilter" :stacked="isClipboardStacked" />
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
  background: #f5f5f5;
  color: #333;
}

/* 深色模式 */
html.dark, html.dark body, html.dark #app {
  background: #1a1a1a;
  color: #e0e0e0;
}

.app-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 12px;
  overflow: hidden;
  background: #f5f5f5;
}

/* 侧边栏布局 */
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

/* 全局 Header（最上方，全宽） */
.global-header {
  position: relative;
  flex-shrink: 0;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
  padding: 12px 12px 0 12px;
}

html.dark .global-header {
  background: #1a1a1a;
  border-bottom-color: #333;
}

.global-header .header {
  padding-bottom: 8px;
}

.global-header .search-wrapper {
  padding: 4px 0 8px;
}

/* 侧边栏 + 内容区 */
.body-area {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.sidebar {
  width: 52px;
  flex-shrink: 0;
  background: #e8e8e8;
  display: flex;
  flex-direction: column;
  padding: 8px 0;
  -webkit-app-region: drag;
  app-region: drag;
  z-index: 10;
  border-right: 1px solid #d0d0d0;
}

html.dark .sidebar {
  background: #1a1a1a;
  border-right-color: #333;
}

.sidebar-buttons {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding-top: 4px;
  flex: 1;
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

.sidebar-spacer {
  flex: 1;
}

.main-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
  padding: 0 12px;
}

.panel.compact-panel {
  padding: 12px;
}

.tasks-panel .task-list,
.clipboard-panel .clipboard-list {
  padding-top: 12px;
  padding-bottom: 12px;
}

html.dark .app-container {
  background: #1a1a1a;
}

/* 窗口拖拽区域 - Tauri v2 */
[data-tauri-drag-region] {
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

/* Mac 原生装饰适配 */
html.platform-mac .header {
  padding-left: 70px;
  padding-top: 10px;
}

html.platform-mac .app-container {
  border-radius: 10px;
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

/* 分类标签区域 - 通过 JS mousedown 处理拖拽，避免与 data-tauri-drag-region 冲突 */
.tabs-wrapper {
  cursor: default;
}

/* 标签按钮本身可点击 */
.tabs-wrapper .tab-btn {
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
  gap: 2px;
}

.win-controls .win-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 0;
}

.win-controls .win-btn:hover {
  background: rgba(0, 0, 0, 0.1);
}

html.dark .win-controls .win-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.win-controls .close-btn:hover {
  background: #e81123 !important;
  color: #fff !important;
}

.tabs-wrapper {
  flex: 1;
  overflow: hidden;
}

.search-wrapper {
  padding: 4px 0 8px;
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

/* 精简模式剪贴板分类 pill */
.compact-clip-filter {
  display: flex;
  gap: 4px;
  padding: 2px 0 6px;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.clip-pill {
  padding: 2px 10px;
  border: 1px solid #d0d0d0;
  border-radius: 12px;
  background: transparent;
  color: #666;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  line-height: 1.4;
}

.clip-pill:hover {
  background: rgba(74, 144, 217, 0.1);
  border-color: #4A90D9;
  color: #4A90D9;
}

.clip-pill.active {
  background: var(--pill-color, #4A90D9);
  border-color: var(--pill-color, #4A90D9);
  color: #fff;
  font-weight: 600;
}

.clip-pill:first-child.active {
  background: #4A90D9;
  border-color: #4A90D9;
}

html.dark .clip-pill {
  border-color: #444;
  color: #999;
}

html.dark .clip-pill:hover {
  background: rgba(74, 144, 217, 0.15);
  border-color: #4A90D9;
  color: #4A90D9;
}

html.dark .clip-pill.active {
  color: #F87171;
  background: rgba(248, 113, 113, 0.15);
  border-color: #F87171;
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