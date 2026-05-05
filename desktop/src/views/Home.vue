<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import {
  NConfigProvider, NMessageProvider, NDialogProvider, NButton, NIcon, NSpace,
  darkTheme
} from 'naive-ui';
import {
  AddOutline as AddIcon, SettingsOutline as SettingsIcon,
  SunnyOutline as LightIcon, MoonOutline as DarkIcon,
  FolderOutline as FolderIcon
} from '@vicons/ionicons5';
import draggable from 'vuedraggable';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { useCategoryStore } from '../stores/categoryStore';
import { useTaskStore } from '../stores/taskStore';
import { useSettingsStore } from '../stores/settingsStore';
import CategoryTabs from '../components/CategoryTabs.vue';
import CategoryPage from '../components/CategoryPage.vue';
import SettingsPage from '../components/SettingsPage.vue';
import SearchBar from '../components/SearchBar.vue';
import TaskCard from '../components/TaskCard.vue';
import TaskFormModal from '../components/TaskFormModal.vue';
import type { Task } from '../types';

const categoryStore = useCategoryStore();
const taskStore = useTaskStore();
const settingsStore = useSettingsStore();
const appWindow = getCurrentWindow();

// 页面切换
const currentPage = ref<'main' | 'category' | 'settings'>('main');

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

  // 置顶任务优先，然后按 sortOrder 排序
  tasks.sort((a, b) => {
    // 置顶的排在前面
    if (a.isPinned !== b.isPinned) {
      return a.isPinned ? -1 : 1;
    }
    // 同级别按 sortOrder 排序
    return a.sortOrder - b.sortOrder;
  });

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
    settingsStore.load()
  ]);

  // 应用保存的窗口尺寸
  applyWindowSize();

  // 监听窗口尺寸变化
  await setupResizeListener();
});

onUnmounted(() => {
  removeResizeListener();
});

// 应用保存的窗口尺寸
async function applyWindowSize() {
  const { windowWidth, windowHeight } = settingsStore.settings;
  if (windowWidth && windowHeight) {
    try {
      await appWindow.setSize(new LogicalSize(windowWidth, windowHeight));
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
    } catch (_) {}
  }, 500); // 500ms 防抖
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

const isDark = computed(() => {
  if (settingsStore.settings.themeMode === 'dark') return true;
  if (settingsStore.settings.themeMode === 'light') return false;
  // system
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
});

// 同步 dark 类到 html 元素
watch(isDark, (val) => {
  document.documentElement.classList.toggle('dark', val);
}, { immediate: true });

function toggleTheme() {
  const next = isDark.value ? 'light' : 'dark';
  settingsStore.setTheme(next);
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

// 置顶/取消置顶任务
function toggleTaskPin(task: Task) {
  task.isPinned = !task.isPinned;
  taskStore.update(task);
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

// 打开分类管理
function openCategoryPage() {
  currentPage.value = 'category';
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
  if (target.closest('button, a, input, select, textarea, [role="button"], .n-button, .n-input, .n-checkbox, .n-switch, .tabs-wrapper')) {
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

const themeOverrides = {
  common: {
    primaryColor: '#4A90D9',
    primaryColorHover: '#5BA4F5',
    borderRadius: '8px'
  }
};
</script>

<template>
  <NConfigProvider :theme="isDark ? darkTheme : null" :themeOverrides="themeOverrides">
    <NMessageProvider>
      <NDialogProvider>
        <!-- 主页面 -->
        <div v-if="currentPage === 'main'" class="app-container">
        <!-- 固定顶部区域（可拖拽窗口） -->
        <div class="fixed-header" @mousedown="startWindowDrag">
          <!-- 标题栏（可拖拽区域） -->
          <div class="header" data-tauri-drag-region>
            <div class="window-controls">
              <span class="dot close" @click="hideToTray()" />
              <span class="dot minimize" @click="appWindow.minimize()" />
              <span class="dot maximize" @click="appWindow.maximize()" />
            </div>
            <div class="tabs-wrapper">
              <CategoryTabs />
            </div>
            <NSpace :size="4">
              <NButton quaternary size="tiny" @click="openCategoryPage">
                <template #icon>
                  <NIcon :component="FolderIcon" />
                </template>
              </NButton>
              <NButton quaternary size="tiny" @click="toggleTheme">
                <template #icon>
                  <NIcon :component="isDark ? LightIcon : DarkIcon" />
                </template>
              </NButton>
              <NButton quaternary size="tiny" @click="openSettingsPage">
                <template #icon>
                  <NIcon :component="SettingsIcon" />
                </template>
              </NButton>
            </NSpace>
          </div>

          <!-- 搜索栏 -->
          <div class="search-wrapper">
            <SearchBar />
          </div>
        </div>

        <!-- 任务列表（可滚动区域） -->
        <div class="task-list" ref="taskListRef">
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
                  @edit="editTask"
                  @delete="deleteTask"
                  @toggle-status="toggleTaskStatus"
                  @toggle-pin="toggleTaskPin"
                  @update-priority="updateTaskPriority"
                  @update-category="updateTaskCategory"
                  @update-start-date="updateTaskStartDate"
                  @update-due-date="updateTaskDueDate"
                  @update-title="updateTaskTitle"
                  @update-description="updateTaskDescription"
                />
              </div>
            </template>
          </draggable>
        </div>

        <!-- 底部添加按钮（固定，可拖拽窗口） -->
        <div class="footer" @mousedown="startWindowDrag">
          <NButton type="primary" block @click="openAddTask">
            <template #icon>
              <NIcon :component="AddIcon" />
            </template>
            添加任务
          </NButton>
        </div>

        <!-- 弹窗 -->
        <TaskFormModal
          :show="showTaskForm"
          :task="editingTask"
          @close="showTaskForm = false"
          @saved="onTaskSaved"
        />
      </div>

        <!-- 分类管理页面 -->
        <CategoryPage v-if="currentPage === 'category'" @back="goBackToMain" />

        <!-- 设置页面 -->
        <SettingsPage v-if="currentPage === 'settings'" @back="goBackToMain" />

      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
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
  height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 12px;
  overflow: hidden;
  background: #f5f5f5;
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

/* 固定顶部区域 */
.fixed-header {
  flex-shrink: 0;
  background: #f5f5f5;
  border-bottom: 1px solid #e0e0e0;
  margin: -12px -12px 0 -12px;
  padding: 12px 12px 0 12px;
}

html.dark .fixed-header {
  background: #1a1a1a;
  border-bottom-color: #333;
}

.header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 8px;
}

/* 标题栏内的交互控件不可拖拽 */
.window-controls,
.tabs-wrapper,
.header .n-space {
  -webkit-app-region: no-drag;
  app-region: no-drag;
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

.tabs-wrapper {
  flex: 1;
  overflow: hidden;
}

.search-wrapper {
  padding: 4px 0 8px;
}

/* 任务列表（唯一可滚动区域） */
.task-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
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