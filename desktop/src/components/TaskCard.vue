<script setup lang="ts">
import { ref, computed, h, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { NIcon, NCheckbox, NDropdown, NDatePicker, NPopover, NModal, NButton } from 'naive-ui';
import {
  ChevronDownOutline as ExpandIcon,
  ChevronUpOutline as TopIcon,
  CreateOutline as EditIcon,
  TrashOutline as DeleteIcon,
  FolderOutline as FolderIcon,
  CalendarOutline as CalendarIcon,
  TimeOutline as ClockIcon,
  ImageOutline as ImageIcon,
  AddOutline as ZoomInIcon,
  RemoveOutline as ZoomOutIcon,
  DownloadOutline as DownloadIcon,
  SyncOutline as ResetIcon,
  SwapHorizontalOutline as FlipHIcon,
  SwapVerticalOutline as FlipVIcon,
  RefreshOutline as RotateIcon,
  AddOutline as PlusIcon
} from '@vicons/ionicons5';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { readFile, exists } from '@tauri-apps/plugin-fs';
import { invoke } from '@tauri-apps/api/core';
import type { Task, Category } from '../types';

const props = defineProps<{
  task: Task;
  categoryColor?: string;
  categories: Category[];
  isEditingTitle?: boolean;
  compact?: boolean;
}>();
const emit = defineEmits<{
  (e: 'edit', task: Task): void;
  (e: 'delete', id: string): void;
  (e: 'toggleStatus', task: Task): void;
  (e: 'updatePriority', task: Task, priority: 1 | 2 | 3): void;
  (e: 'updateCategory', task: Task, categoryId: string): void;
  (e: 'updateStartDate', task: Task, date: number | undefined): void;
  (e: 'updateDueDate', task: Task, date: number | undefined): void;
  (e: 'updateTitle', task: Task, title: string): void;
  (e: 'updateDescription', task: Task, description: string | undefined): void;
  (e: 'updateThumbnail', task: Task, thumbnail: string | undefined): void;
  (e: 'moveToTop', task: Task): void;
  (e: 'editingDescChange', isEditing: boolean): void;
  (e: 'editingTitleChange', isEditing: boolean): void;
}>();

// 截图提示状态
const screenshotHint = ref(false);
const screenshotHintText = ref('');

const isDone = computed(() => props.task.status === 'done');

// 标题编辑状态
const isEditing = ref(false);
const editTitleValue = ref('');
const titleInputRef = ref<HTMLInputElement | null>(null);

// 监听外部编辑状态（新添加的任务自动进入编辑）
watch(() => props.isEditingTitle, (val) => {
  if (val && !isEditing.value) {
    startEditTitle();
  }
});

// 开始编辑标题
function startEditTitle() {
  isEditing.value = true;
  emit('editingTitleChange', true);
  // 如果是默认文字，清空；否则保留原标题
  editTitleValue.value = props.task.title === '待输入任务内容……' ? '' : props.task.title;
  nextTick(() => {
    titleInputRef.value?.focus();
  });
}

// 保存标题
function saveTitle() {
  const newTitle = editTitleValue.value.trim() || '待输入任务内容……';
  emit('updateTitle', props.task, newTitle);
  isEditing.value = false;
  emit('editingTitleChange', false);
}

// 取消编辑标题
function cancelEdit() {
  isEditing.value = false;
  editTitleValue.value = '';
  emit('editingTitleChange', false);
}

// 描述编辑状态
const isEditingDesc = ref(false);
const editDescValue = ref('');
const descInputRef = ref<HTMLTextAreaElement | null>(null);

// 编辑时 textarea 的动态行数（最多 20 行）
const editRows = computed(() => {
  if (!isEditingDesc.value || !editDescValue.value) return 3;
  const lines = editDescValue.value.split('\n').length;
  return Math.min(lines, 20);
});

// 开始编辑描述
function startEditDesc() {
  isEditingDesc.value = true;
  isExpanded.value = true;
  emit('editingDescChange', true);
  // 如果没有描述，初始化第一行编号
  if (!props.task.description) {
    editDescValue.value = '1、';
  } else {
    editDescValue.value = props.task.description;
  }
  nextTick(() => {
    descInputRef.value?.focus();
  });
}

// 保存描述
function saveDesc() {
  const newDesc = editDescValue.value.trim() || undefined;
  emit('updateDescription', props.task, newDesc);
  isEditingDesc.value = false;
  emit('editingDescChange', false);
}

// 取消编辑描述
function cancelDescEdit() {
  isEditingDesc.value = false;
  editDescValue.value = '';
  emit('editingDescChange', false);
}

// 按下回车时，在新行添加下一个编号
function handleDescKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    const lines = editDescValue.value.split('\n');
    const nextNum = lines.length + 1;
    editDescValue.value += `\n${nextNum}、`;
  }
}

// 精简模式下根据优先级返回底色 class
const priorityBgClass = computed(() => {
  if (!props.compact) return '';
  if (props.task.priority === 1) return 'priority-high-bg';
  if (props.task.priority === 2) return 'priority-medium-bg';
  return 'priority-low-bg';
});

// 点击圆点循环切换优先级
function handleSetPriority(starIndex: number) {
  const newPriority = starIndex as 1 | 2 | 3;
  emit('updatePriority', props.task, newPriority);
}

// 倒计天数计算
const countdownDays = computed(() => {
  if (!props.task.dueDate) return null;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dueDate = new Date(props.task.dueDate);
  dueDate.setHours(0, 0, 0, 0);
  const diffDays = Math.ceil((dueDate.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));
  return diffDays;
});

// 倒计天数显示文本和样式
const countdownText = computed(() => {
  if (countdownDays.value === null) return '';
  if (countdownDays.value < 0) return '已过期';
  if (countdownDays.value === 0) return '今天';
  if (countdownDays.value === 1) return '明天';
  return `${countdownDays.value}天`;
});

const countdownClass = computed(() => {
  if (countdownDays.value === null) return '';
  if (countdownDays.value < 0) return 'overdue';
  if (countdownDays.value <= 1) return 'urgent';
  if (countdownDays.value <= 3) return 'soon';
  return 'normal';
});

// 格式化日期显示
function formatDate(timestamp: number | undefined): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  const month = date.getMonth() + 1;
  const day = date.getDate();
  return `${month}/${day}`;
}

// 获取当前分类名称
const currentCategoryName = computed(() => {
  const cat = props.categories.find(c => c.id === props.task.categoryId);
  return cat?.name || '';
});

// 更新开始日期
function handleStartDateChange(value: number | null) {
  emit('updateStartDate', props.task, value ?? undefined);
}

// 更新截止日期
function handleDueDateChange(value: number | null) {
  emit('updateDueDate', props.task, value ?? undefined);
}

// 右键菜单选项
const contextMenuOptions = computed(() => {
  const categoryOptions = props.categories.map(cat => ({
    label: cat.name,
    key: `cat-${cat.id}`,
    icon: () => h(NIcon, {
      component: FolderIcon,
      size: 16,
      style: { color: cat.color }
    })
  }));

  return [
    {
      label: '编辑',
      key: 'edit',
      icon: () => h(NIcon, { component: EditIcon, size: 16 })
    },
    {
      label: '选择图片',
      key: 'image',
      icon: () => h(NIcon, { component: ImageIcon, size: 16 })
    },
    {
      label: '截图存为缩略图',
      key: 'screenshot',
      icon: () => h(NIcon, { component: ImageIcon, size: 16 })
    },
    {
      label: props.task.thumbnailBase64 ? '清除图片' : '',
      key: 'clearImage',
      icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } }),
      show: !!props.task.thumbnailBase64
    },
    {
      label: '删除',
      key: 'delete',
      icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } })
    },
    {
      label: '移动到最顶部',
      key: 'moveToTop',
      icon: () => h(NIcon, { component: TopIcon, size: 16 })
    },
    { type: 'divider', key: 'd1' },
    ...categoryOptions
  ];
});

// 右键菜单显示状态
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuPlacement = ref<'bottom-start' | 'top-start'>('bottom-start');

// 右键点击显示菜单
function handleContextMenu(e: MouseEvent) {
  e.preventDefault();

  // 获取窗口尺寸和预估菜单尺寸
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  // 预估菜单宽度
  const estimatedMenuWidth = 220;
  // 预估菜单高度（每个选项约 36px，最大 400px）
  const estimatedMenuHeight = Math.min(contextMenuOptions.value.length * 36, 400);

  // 计算调整后的位置
  let x = e.clientX;
  let y = e.clientY;

  // 动态选择 placement：如果底部空间不足，向上显示
  const bottomSpace = windowHeight - y;
  contextMenuPlacement.value = bottomSpace < estimatedMenuHeight ? 'top-start' : 'bottom-start';

  // 如果菜单会超出右边界，向左偏移
  if (x + estimatedMenuWidth > windowWidth) {
    x = windowWidth - estimatedMenuWidth - 10;
  }

  // 确保不超出左边界
  if (x < 10) {
    x = 10;
  }

  // 确保不超出顶部边界（向上显示时需要调整）
  if (contextMenuPlacement.value === 'top-start' && y < estimatedMenuHeight) {
    y = estimatedMenuHeight + 10;
  }

  contextMenuX.value = x;
  contextMenuY.value = y;
  showContextMenu.value = true;
}

// ESC 键关闭右键菜单
function handleEscKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && showContextMenu.value) {
    showContextMenu.value = false;
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleEscKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleEscKeydown);
});

// 处理菜单选择
function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'edit') {
    emit('edit', props.task);
  } else if (key === 'delete') {
    emit('delete', props.task.id);
  } else if (key === 'image') {
    selectImage();
  } else if (key === 'screenshot') {
    takeScreenshot();
  } else if (key === 'clearImage') {
    emit('updateThumbnail', props.task, undefined);
  } else if (key === 'moveToTop') {
    emit('moveToTop', props.task);
  } else if (key.startsWith('cat-')) {
    const categoryId = key.slice(4);
    if (categoryId !== props.task.categoryId) {
      emit('updateCategory', props.task, categoryId);
    }
  }
}

// 选择图片作为参考图
async function selectImage() {
  try {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{
        name: '图片',
        extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp']
      }]
    });
    if (selected) {
      const fileData = await readFile(selected as string);
      const base64 = uint8ArrayToBase64(fileData);
      emit('updateThumbnail', props.task, base64);
    }
  } catch (e) {
    console.error('选择图片失败:', e);
  }
}

// 启动截图并保存为缩略图
async function takeScreenshot() {
  // 判断操作系统
  const platform = navigator.platform.toLowerCase();
  const isMac = platform.includes('mac');

  if (isMac) {
    // Mac 平台：提示手动截图
    screenshotHintText.value = '请按 Cmd+Shift+4 截图（按住 Control 保存到剪贴板），完成后回到此窗口自动保存';
    screenshotHint.value = true;
    listenForFocus();
    return;
  }

  // Windows 平台：原有逻辑
  let pixpinPath = '';

  // 优先从运行中的进程查找 Pixpin
  try {
    const result = await invoke<string | null>('find_pixpin_path');
    if (result) {
      pixpinPath = result;
    }
  } catch { /* ignore */ }

  // 如果进程没找到，尝试常见安装路径
  if (!pixpinPath) {
    const pixpinPaths = [
      'C:\\Program Files\\Pixpin\\PixPin.exe',
      'C:\\Program Files (x86)\\Pixpin\\PixPin.exe',
    ];
    for (const p of pixpinPaths) {
      try {
        if (await exists(p)) {
          pixpinPath = p;
          break;
        }
      } catch { /* ignore */ }
    }
  }

  if (pixpinPath) {
    // 显示提示
    screenshotHintText.value = '请按 Pixpin 快捷键，或 Win+Shift+S 截图，完成后自动保存';
    screenshotHint.value = true;

    // 用 Rust 后端启动 Pixpin，绕过 shell scope 限制
    await invoke('launch_pixpin', { pixpinPath });

    // 监听窗口焦点变化，截图完成后回到窗口时读取剪贴板
    listenForFocus();
  } else {
    // 没有 Pixpin，提示手动截图
    screenshotHintText.value = '请按 Win+Shift+S 截图，完成后自动保存';
    screenshotHint.value = true;
    listenForFocus();
  }
}

// 监听窗口焦点变化
async function listenForFocus() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const appWindow = getCurrentWindow();

  // 窗口重新获得焦点时读取剪贴板
  const unlistenFocus = await appWindow.onFocusChanged((event) => {
    if (event.payload) {
      console.log('窗口重新获得焦点，准备读取剪贴板');
      setTimeout(() => {
        readClipboardImage();
        screenshotHint.value = false;
      }, 300);
      unlistenFocus();
    }
  });

  // 兜底：5 秒后无论如何都尝试读取一次
  setTimeout(() => {
    if (screenshotHint.value) {
      console.log('超时兜底，尝试读取剪贴板');
      readClipboardImage();
      screenshotHint.value = false;
    }
  }, 5000);
}

// 从剪贴板读取图片并保存
async function readClipboardImage() {
  try {
    // 用 Tauri clipboard-manager 插件读取图片
    const { invoke } = await import('@tauri-apps/api/core');
    // 使用 tauri-plugin-clipboard-manager 的 read_image 命令
    const result = await invoke<{ base64?: string } | string>('plugin:clipboard-manager|read_image');
    if (result) {
      let base64Data = '';
      if (typeof result === 'string') {
        base64Data = result;
      } else if (result.base64) {
        base64Data = result.base64;
      }
      if (base64Data) {
        emit('updateThumbnail', props.task, `data:image/png;base64,${base64Data}`);
        return;
      }
    }
    console.log('clipboard-manager 返回空数据，尝试后端读取');
  } catch (e) {
    console.log('clipboard-manager 读取失败:', e, '尝试后端读取');
  }

  // 回退：用后端读取
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const base64 = await invoke<string | null>('read_clipboard_image');
    if (base64) {
      emit('updateThumbnail', props.task, base64);
    }
  } catch (e) {
    console.error('后端读取剪贴板失败:', e);
  }
}

// Uint8Array 转 Base64
function uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

// 图片预览状态
const showImagePreview = ref(false);
const imageScale = ref(1);
const imageRotate = ref(0);
const flipH = ref(false);
const flipV = ref(false);

function openImagePreview() {
  imageScale.value = 1;
  imageRotate.value = 0;
  flipH.value = false;
  flipV.value = false;
  showImagePreview.value = true;
}

function closeImagePreview() {
  showImagePreview.value = false;
}

function zoomIn() {
  imageScale.value = Math.min(imageScale.value + 0.25, 4);
}

function zoomOut() {
  imageScale.value = Math.max(imageScale.value - 0.25, 0.25);
}

function rotateImage() {
  imageRotate.value = (imageRotate.value + 90) % 360;
}

function flipHorizontal() {
  flipH.value = !flipH.value;
}

function flipVertical() {
  flipV.value = !flipV.value;
}

function resetImage() {
  imageScale.value = 1;
  imageRotate.value = 0;
  flipH.value = false;
  flipV.value = false;
}

async function downloadImage() {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const filePath = await save({
      filters: [{ name: '图片', extensions: ['png'] }]
    });
    if (filePath) {
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      const base64 = props.task.thumbnailBase64!.replace(/^data:image\/\w+;base64,/, '');
      const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
      await writeFile(filePath, bytes);
    }
  } catch (e) {
    console.error('保存图片失败:', e);
  }
}

function deleteThumbnail() {
  emit('updateThumbnail', props.task, undefined);
  closeImagePreview();
}

// 鼠标滚轮缩放
function handleWheel(e: WheelEvent) {
  e.preventDefault();
  if (e.deltaY < 0) zoomIn();
  else zoomOut();
}

// 展开/折叠状态
const isExpanded = ref(false);

// 是否需要展开功能（描述长度判断）
const needsExpand = computed(() =>
  props.task.description && props.task.description.length > 30
);

// 是否有缩略图
const hasThumbnail = computed(() => !!props.task.thumbnailBase64);

// 点击卡片空白区 → 仅切换展开/折叠，不进入编辑
function toggleExpand(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 排除交互元素
  if (target.closest('.n-checkbox, .priority-dot, .star-icon, .expand-icon, .date-picker-trigger, .n-date-picker, .task-title, .title-input, .right-area, .thumbnail-wrapper, .task-desc, .task-desc-empty, .desc-input, .desc-content, .tag, .op-btn, .add-thumb-btn, .checkbox-custom')) return;

  isExpanded.value = !isExpanded.value;
  // 折叠时如果正在编辑，保存
  if (!isExpanded.value && isEditingDesc.value) {
    saveDesc();
  }
}

function handleToggleStatus() {
  emit('toggleStatus', props.task);
}

// 优先级圆点样式
const priorityDotClass = computed(() => {
  if (props.task.priority === 1) return 'p-high';
  if (props.task.priority === 2) return 'p-medium';
  return 'p-low';
});
</script>

<template>
  <!-- 右键菜单 -->
  <NDropdown
    :placement="contextMenuPlacement"
    trigger="manual"
    :x="contextMenuX"
    :y="contextMenuY"
    :show="showContextMenu"
    :options="contextMenuOptions"
    to="body"
    @select="handleMenuSelect"
    @clickoutside="showContextMenu = false"
  />

  <!-- 截图提示 -->
  <div v-if="screenshotHint" class="screenshot-hint">
    {{ screenshotHintText }}
  </div>

  <div
    class="task-card"
    :class="[
      isDone && 'done',
      isExpanded && 'expanded',
      props.compact && 'compact',
      props.compact && priorityBgClass,
      (isEditing || isEditingDesc) && 'editing',
      `p-${props.task.priority}`
    ]"
    @click="toggleExpand"
    @contextmenu="handleContextMenu"
  >
    <!-- 左侧色条 -->
    <div
      v-if="categoryColor && !props.compact"
      class="color-bar"
      :style="{ backgroundColor: categoryColor }"
    ></div>

    <!-- 完成状态复选框（非精简模式） -->
    <NCheckbox
      v-if="!props.compact"
      :checked="isDone"
      size="small"
      @update:checked="handleToggleStatus"
      class="status-checkbox"
    />

    <!-- 精简模式复选框 -->
    <button
      v-else
      class="checkbox-custom"
      :class="{ checked: isDone }"
      @click.stop="handleToggleStatus"
    >
      <svg v-if="isDone" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="20 6 9 17 4 12"/>
      </svg>
    </button>

    <!-- 任务主体 -->
    <div class="task-body">
      <!-- 第一行：优先级 + 标题 + 标签 -->
      <div class="task-row">
        <!-- 优先级圆点 -->
        <span v-if="!props.compact" class="priority-dot" :class="priorityDotClass" @click.stop="handleSetPriority(props.task.priority === 1 ? 3 : (props.task.priority === 2 ? 1 : 2))"></span>

        <!-- 标题 -->
        <div class="title-area">
          <span v-if="!isEditing" class="task-title" @click.stop="startEditTitle">
            {{ props.task.title }}
          </span>
          <input
            v-else
            ref="titleInputRef"
            v-model="editTitleValue"
            class="title-input"
            placeholder="输入任务内容"
            @mousedown.stop
            @blur="saveTitle"
            @keyup.enter="saveTitle"
            @keyup.escape="cancelEdit"
          />
        </div>

        <!-- 标签区域 -->
        <div class="task-tags" v-if="!props.compact">
          <!-- 分类标签 -->
          <span v-if="currentCategoryName" class="tag category-tag" :style="{ borderLeftColor: categoryColor }">
            {{ currentCategoryName }}
          </span>

          <!-- 开始日期 -->
          <NPopover v-if="props.task.startDate" trigger="click" placement="bottom" :show-arrow="false">
            <template #trigger>
              <span class="tag date-tag">
                <NIcon :component="CalendarIcon" size="10" />
                {{ formatDate(props.task.startDate) }}
              </span>
            </template>
            <NDatePicker
              :value="props.task.startDate ?? null"
              type="date"
              size="small"
              :default-value="Date.now()"
              @update:value="handleStartDateChange"
              style="width: 200px"
            />
          </NPopover>

          <!-- 截止日期 -->
          <NPopover trigger="click" placement="bottom" :show-arrow="false">
            <template #trigger>
              <span class="tag date-tag" :class="countdownClass">
                <NIcon :component="ClockIcon" size="10" />
                {{ formatDate(props.task.dueDate) || '截止' }}
              </span>
            </template>
            <NDatePicker
              :value="props.task.dueDate ?? null"
              type="date"
              size="small"
              :default-value="props.task.startDate ?? Date.now()"
              @update:value="handleDueDateChange"
              style="width: 200px"
            />
          </NPopover>

          <!-- 倒计时 -->
          <span v-if="countdownText" class="tag countdown-tag" :class="countdownClass">
            {{ countdownText }}
          </span>
        </div>

        <!-- 精简模式倒计时 -->
        <span v-if="props.compact && countdownText" class="compact-countdown" :class="countdownClass">
          {{ countdownText }}
        </span>

        <!-- 操作按钮 -->
        <div class="task-ops" v-if="!props.compact">
          <button class="op-btn" title="置顶" @click.stop="emit('moveToTop', props.task)">
            <NIcon :component="TopIcon" size="14" />
          </button>
          <button class="op-btn" title="编辑" @click.stop="startEditTitle">
            <NIcon :component="EditIcon" size="14" />
          </button>
          <button class="op-btn del" title="删除" @click.stop="emit('delete', props.task.id)">
            <NIcon :component="DeleteIcon" size="14" />
          </button>
        </div>

        <!-- 展开/折叠按钮 -->
        <button
          v-if="(needsExpand || hasThumbnail || props.task.description) && !props.compact"
          class="expand-btn"
          :class="{ expanded: isExpanded }"
          @click.stop="isExpanded = !isExpanded"
        >
          <NIcon :component="ExpandIcon" size="14" />
        </button>
      </div>

      <!-- 展开区域：描述 + 缩略图 -->
      <div v-if="isExpanded && !props.compact" class="expanded-area">
        <!-- 描述 -->
        <div class="task-desc-wrapper">
          <div v-if="!isEditingDesc && props.task.description" class="desc-content" @click.stop="startEditDesc">
            {{ props.task.description }}
          </div>
          <div v-else-if="!isEditingDesc" class="desc-hint" @click.stop="startEditDesc">
            点击添加描述...
          </div>
          <textarea
            v-else
            ref="descInputRef"
            v-model="editDescValue"
            class="desc-input"
            placeholder="添加描述..."
            :rows="editRows"
            @blur="saveDesc"
            @keydown="handleDescKeydown"
            @keyup.escape="cancelDescEdit"
            @mousedown.stop
            @click.stop
          />
        </div>

        <!-- 缩略图区域 -->
        <div class="task-thumbs">
          <div v-if="hasThumbnail" class="thumb-item" @click.stop="openImagePreview">
            <img
              :src="props.task.thumbnailBase64?.startsWith('data:') ? props.task.thumbnailBase64 : `data:image/jpeg;base64,${props.task.thumbnailBase64}`"
              class="thumb-img"
              alt=""
            />
          </div>
          <button class="add-thumb-btn" @click.stop="selectImage" title="添加图片">
            <NIcon :component="PlusIcon" size="18" />
          </button>
        </div>
      </div>

      <!-- 非展开时，有描述显示预览 -->
      <div
        v-if="!isExpanded && props.task.description && !props.compact"
        class="desc-preview"
        @click.stop="isExpanded = true"
      >
        {{ props.task.description }}
      </div>
    </div>
  </div>

  <!-- 图片预览弹窗 -->
  <NModal v-model:show="showImagePreview" :mask-closable="true" @after-leave="closeImagePreview" @keydown.esc="closeImagePreview">
    <div class="image-preview-overlay" @click="closeImagePreview" @wheel="handleWheel">
      <div class="image-preview-container" @click.stop>
        <img
          :src="props.task.thumbnailBase64?.startsWith('data:') ? props.task.thumbnailBase64 : `data:image/jpeg;base64,${props.task.thumbnailBase64}`"
          class="preview-image"
          :style="{ transform: `scale(${flipH ? -1 : 1} * ${imageScale}, ${flipV ? -1 : 1} * ${imageScale}) rotate(${imageRotate}deg)` }"
          @click="closeImagePreview"
          alt=""
        />
      </div>
      <div class="preview-toolbar" @click.stop>
        <NButton quaternary circle @click="zoomOut" title="缩小">
          <NIcon :component="ZoomOutIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="zoomIn" title="放大">
          <NIcon :component="ZoomInIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="resetImage" title="重置">
          <NIcon :component="ResetIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="flipHorizontal" title="水平翻转">
          <NIcon :component="FlipHIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="flipVertical" title="垂直翻转">
          <NIcon :component="FlipVIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="rotateImage" title="旋转">
          <NIcon :component="RotateIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="downloadImage" title="保存">
          <NIcon :component="DownloadIcon" size="18" />
        </NButton>
        <NButton quaternary circle @click="deleteThumbnail" title="删除">
          <NIcon :component="DeleteIcon" size="18" style="color:#E05252" />
        </NButton>
      </div>
    </div>
  </NModal>
</template>

<style scoped>
.task-card {
  --card-bg: #ffffff;
  --card-hover: #fafafa;
  --text-primary: #1a1a1a;
  --text-secondary: #888888;
  --text-muted: #aaaaaa;
  --accent: #4A90D9;
  --accent-light: rgba(74, 144, 217, 0.1);
  --border: #e8e8e8;
  --danger: #E05252;
  --radius-md: 12px;
  --radius-full: 999px;
  --p-high: #E05252;
  --p-medium: #FF9800;
  --p-low: transparent;

  width: 100%;
  padding: 12px 14px;
  background: var(--card-bg);
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  user-select: none;
  display: flex;
  align-items: flex-start;
  font-family: var(--task-font-family, inherit);
  gap: 10px;
  position: relative;
  cursor: pointer;
  margin-bottom: 6px;
}

html.dark .task-card {
  --card-bg: #2a2a2a;
  --card-hover: #303030;
  --text-primary: #f0f0f0;
  --text-secondary: #888;
  --text-muted: #555;
  --accent: #5BA4F5;
  --accent-light: rgba(91, 164, 245, 0.15);
  --border: #3a3a3a;
}

.task-card:hover {
  border-color: rgba(74, 144, 217, 0.4);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  background: var(--card-hover);
}

html.dark .task-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.task-card.editing {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.task-card.done {
  opacity: 0.5;
}

.task-card.done .task-title {
  text-decoration: line-through;
  color: var(--text-muted);
}

/* 左侧色条 */
.color-bar {
  width: 3px;
  height: 50px;
  border-radius: 2px;
  flex-shrink: 0;
  position: absolute;
  left: 0;
  top: 12px;
}

/* 复选框 */
.status-checkbox {
  flex-shrink: 0;
  margin-top: 2px;
  z-index: 1;
}

.checkbox-custom {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--border);
  flex-shrink: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  margin-top: 2px;
  background: transparent;
  padding: 0;
  z-index: 1;
}

.checkbox-custom:hover {
  border-color: #28C840;
  background: rgba(40, 200, 64, 0.1);
}

.checkbox-custom.checked {
  background: #28C840;
  border-color: #28C840;
}

.checkbox-custom svg {
  width: 12px;
  height: 12px;
  color: white;
}

/* 任务主体 */
.task-body {
  flex: 1;
  min-width: 0;
  padding-left: 0;
}

.task-card .color-bar + .status-checkbox,
.task-card .color-bar + .checkbox-custom {
  margin-left: 6px;
}

/* 第一行 */
.task-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

/* 优先级圆点 */
.priority-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 6px;
  cursor: pointer;
  transition: transform 0.15s;
}

.priority-dot:hover {
  transform: scale(1.3);
}

.priority-dot.p-high {
  background: var(--p-high);
}

.priority-dot.p-medium {
  background: var(--p-medium);
}

.priority-dot.p-low {
  background: var(--border);
}

/* 标题区域 */
.title-area {
  flex: 1;
  min-width: 0;
}

.task-title {
  font-size: 14px;
  font-weight: 500;
  line-height: 1.5;
  color: var(--text-primary);
  word-break: break-word;
  cursor: text;
}

.title-input {
  font-size: 14px;
  font-weight: 500;
  line-height: 1.5;
  color: var(--text-primary);
  background: transparent;
  border: none;
  border-bottom: 2px solid var(--accent);
  outline: none;
  width: 100%;
  padding: 0;
  font-family: inherit;
}

.title-input::placeholder {
  color: var(--text-muted);
}

/* 标签区域 */
.task-tags {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  opacity: 0.6;
  transition: opacity 0.15s;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.task-card:hover .task-tags {
  opacity: 1;
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  color: var(--text-secondary);
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: rgba(0, 0, 0, 0.04);
  white-space: nowrap;
  cursor: pointer;
  transition: all 0.15s;
  border-left: 3px solid transparent;
}

html.dark .tag {
  background: rgba(255, 255, 255, 0.06);
}

.tag:hover {
  background: var(--accent-light);
  color: var(--accent);
}

.category-tag {
  padding-left: 5px;
}

.date-tag {
  border-left: none;
}

.countdown-tag.normal {
  background: rgba(40, 200, 64, 0.1);
  color: #28C840;
}

.countdown-tag.soon {
  background: rgba(255, 184, 0, 0.15);
  color: #FFB800;
}

.countdown-tag.urgent {
  background: rgba(224, 82, 82, 0.15);
  color: var(--danger);
  font-weight: 600;
  animation: pulse-tag 2s infinite;
}

.countdown-tag.overdue {
  background: rgba(224, 82, 82, 0.15);
  color: var(--danger);
  font-weight: 600;
}

html.dark .countdown-tag.normal {
  background: rgba(40, 200, 64, 0.2);
}

html.dark .countdown-tag.soon {
  background: rgba(255, 184, 0, 0.2);
}

html.dark .countdown-tag.urgent,
html.dark .countdown-tag.overdue {
  background: rgba(224, 82, 82, 0.25);
}

@keyframes pulse-tag {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

/* 精简模式倒计时 */
.compact-countdown {
  padding: 3px 10px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  margin-left: 12px;
  flex-shrink: 0;
}

.compact-countdown.normal {
  background: #e8f5e9;
  color: #28C840;
}

.compact-countdown.soon {
  background: rgba(224, 82, 82, 0.12);
  color: #E05252;
}

.compact-countdown.urgent {
  background: #ffebee;
  color: #E05252;
  animation: pulse 1.5s ease-in-out infinite;
}

.compact-countdown.overdue {
  background: #f5f5f5;
  color: #999;
}

html.dark .compact-countdown.normal {
  background: rgba(40, 200, 64, 0.2);
  color: #4ade80;
}

html.dark .compact-countdown.soon {
  background: rgba(224, 82, 82, 0.25);
  color: #f87171;
}

html.dark .compact-countdown.urgent {
  background: rgba(224, 82, 82, 0.2);
  color: #f87171;
  animation: pulse 1.5s ease-in-out infinite;
}

html.dark .compact-countdown.overdue {
  background: rgba(100, 100, 100, 0.2);
  color: #888;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

/* 操作按钮 */
.task-ops {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
  flex-shrink: 0;
}

.task-card:hover .task-ops {
  opacity: 1;
}

.op-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  padding: 0;
}

.op-btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: var(--text-primary);
}

html.dark .op-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

.op-btn.del:hover {
  background: rgba(224, 82, 82, 0.1);
  color: var(--danger);
}

/* 展开按钮 */
.expand-btn {
  width: 22px;
  height: 22px;
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s, color 0.15s;
  flex-shrink: 0;
  margin-top: 1px;
  padding: 0;
  border-radius: 4px;
}

.expand-btn:hover {
  color: var(--accent);
  background: rgba(0, 0, 0, 0.04);
}

html.dark .expand-btn:hover {
  background: rgba(255, 255, 255, 0.06);
}

.expand-btn.expanded {
  transform: rotate(180deg);
}

/* 展开区域 */
.expanded-area {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 描述 */
.task-desc-wrapper {
  padding: 8px 10px;
  background: rgba(0, 0, 0, 0.02);
  border-radius: var(--radius-sm, 8px);
}

html.dark .task-desc-wrapper {
  background: rgba(255, 255, 255, 0.03);
}

.desc-content {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  cursor: text;
}

.desc-hint {
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
}

.desc-hint:hover {
  color: var(--accent);
}

.desc-input {
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px dashed var(--accent);
  outline: none;
  width: 100%;
  padding: 4px 6px;
  font-family: inherit;
  resize: none;
  line-height: 1.6;
  border-radius: 4px;
  background: var(--accent-light);
}

.desc-input::placeholder {
  color: var(--text-muted);
}

/* 描述预览（折叠时） */
.desc-preview {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  padding-left: 16px;
}

.desc-preview:hover {
  color: var(--text-secondary);
}

/* 缩略图区域 */
.task-thumbs {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.thumb-item {
  width: 80px;
  height: 56px;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--bg, #f5f5f0);
  transition: transform 0.15s, box-shadow 0.15s;
}

.thumb-item:hover {
  transform: scale(1.05);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  z-index: 1;
}

html.dark .thumb-item:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.add-thumb-btn {
  width: 80px;
  height: 56px;
  border-radius: 8px;
  border: 2px dashed var(--border);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  padding: 0;
}

.add-thumb-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-light);
}

/* 精简模式样式 */
.task-card.compact {
  padding: 8px 10px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(74, 144, 217, 0.2);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  gap: 8px;
}

.task-card.compact .color-bar {
  display: none;
}

html.dark .task-card.compact {
  background: rgba(42, 42, 42, 0.95);
  border-color: rgba(91, 164, 245, 0.25);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
}

.task-card.compact:hover {
  transform: none;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
}

html.dark .task-card.compact:hover {
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.25);
}

.task-card.compact .task-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

/* 精简模式 - 优先级底色 */
.task-card.compact.priority-high-bg {
  background: rgba(255, 248, 225, 0.95);
  border-color: rgba(255, 183, 77, 0.4);
}

html.dark .task-card.compact.priority-high-bg {
  background: rgba(62, 39, 35, 0.95);
  border-color: rgba(255, 183, 77, 0.35);
}

.task-card.compact.priority-medium-bg {
  background: rgba(227, 242, 253, 0.95);
  border-color: rgba(100, 181, 246, 0.4);
}

html.dark .task-card.compact.priority-medium-bg {
  background: rgba(25, 40, 60, 0.95);
  border-color: rgba(100, 181, 246, 0.35);
}

.task-card.compact.priority-low-bg {
  background: rgba(232, 245, 233, 0.95);
  border-color: rgba(102, 187, 106, 0.4);
}

html.dark .task-card.compact.priority-low-bg {
  background: rgba(27, 45, 35, 0.95);
  border-color: rgba(102, 187, 106, 0.35);
}

/* 截图提示 */
.screenshot-hint {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.75);
  color: #fff;
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 13px;
  z-index: 9999;
  pointer-events: none;
  animation: fadeIn 0.2s ease;
}

html.dark .screenshot-hint {
  background: rgba(50, 50, 50, 0.9);
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(8px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}

/* 图片预览 */
.image-preview-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  cursor: zoom-out;
}

.image-preview-container {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  width: 100%;
  overflow: hidden;
}

.preview-image {
  max-width: 90vw;
  max-height: 80vh;
  object-fit: contain;
  transition: transform 0.2s ease;
  cursor: grab;
  user-select: none;
}

.preview-toolbar {
  display: flex;
  gap: 4px;
  padding: 12px 20px;
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border-radius: 12px;
  margin-bottom: 20px;
}

html.dark .preview-toolbar {
  background: rgba(40, 40, 40, 0.6);
}

.preview-toolbar .n-button {
  color: #fff;
}

html.dark .preview-toolbar .n-button {
  color: #e0e0e0;
}

.preview-toolbar .n-button:hover {
  background: rgba(255, 255, 255, 0.15);
}
</style>
