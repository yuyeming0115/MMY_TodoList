<script setup lang="ts">
import { ref, computed, h, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { NIcon, NCheckbox, NDropdown, NDatePicker, NPopover, NModal, NButton } from 'naive-ui';
import {
  StarOutline as StarOutlineIcon,
  Star as StarIcon,
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
  RefreshOutline as RotateIcon
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
}

// 取消编辑标题
function cancelEdit() {
  isEditing.value = false;
  editTitleValue.value = '';
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
}

// 取消编辑描述
function cancelDescEdit() {
  isEditingDesc.value = false;
  editDescValue.value = '';
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

// 优先级星级 (1=高=3星, 2=中=2星, 3=低=1星)
// 星标优先级显示（非精简模式使用）
const priorityStars = computed(() => props.task.priority);

// 精简模式下根据优先级返回底色 class
const priorityBgClass = computed(() => {
  if (!props.compact) return '';
  if (props.task.priority === 1) return 'priority-high-bg'; // 高优先级 - 暖色
  if (props.task.priority === 2) return 'priority-medium-bg'; // 中优先级 - 蓝色
  return 'priority-low-bg'; // 低优先级 - 默认
});

// 点击星星设置优先级
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

// 右键点击显示菜单
function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
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

// 点击卡片空白区 → 仅切换展开/折叠，不进入编辑
function toggleExpand(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 排除交互元素
  if (target.closest('.n-checkbox, .star-icon, .expand-icon, .date-picker-trigger, .n-date-picker, .task-title, .title-input, .right-area, .thumbnail-wrapper, .task-desc, .task-desc-empty, .desc-input, .desc-content')) return;

  isExpanded.value = !isExpanded.value;
  // 折叠时如果正在编辑，保存
  if (!isExpanded.value && isEditingDesc.value) {
    saveDesc();
  }
}

function handleToggleStatus() {
  emit('toggleStatus', props.task);
}
</script>

<template>
  <!-- 右键菜单 -->
  <NDropdown
    placement="bottom-start"
    trigger="manual"
    :x="contextMenuX"
    :y="contextMenuY"
    :show="showContextMenu"
    :options="contextMenuOptions"
    @select="handleMenuSelect"
    @clickoutside="showContextMenu = false"
  />

  <!-- 截图提示 -->
  <div v-if="screenshotHint" class="screenshot-hint">
    {{ screenshotHintText }}
  </div>
  <div
    class="simple-card"
    :class="[isDone && 'done', isExpanded && 'expanded', props.compact && 'compact', props.compact && priorityBgClass]"
    :style="{ borderLeftColor: categoryColor || 'transparent', borderLeftWidth: (categoryColor && !props.compact) ? '3px' : '0' }"
    @click="toggleExpand"
    @contextmenu="handleContextMenu"
  >
    <!-- 完成状态复选框 -->
    <NCheckbox
      v-if="!props.compact"
      :checked="isDone"
      size="small"
      @update:checked="handleToggleStatus"
      class="status-checkbox"
    />

    <!-- 缩略图（如果有） -->
    <div v-if="props.task.thumbnailBase64 && !props.compact" class="thumbnail-wrapper" @click.stop="openImagePreview">
      <img
        :src="`data:image/jpeg;base64,${props.task.thumbnailBase64}`"
        class="thumbnail"
        alt=""
      />
    </div>

    <!-- 任务内容 -->
    <div class="task-content">
      <div class="task-header">
        <!-- 标题（可编辑） -->
        <span v-if="!isEditing" class="task-title" @click="startEditTitle">
          {{ props.task.title }}
        </span>
        <input
          v-else
          ref="titleInputRef"
          v-model="editTitleValue"
          class="title-input"
          placeholder="输入任务内容"
          @blur="saveTitle"
          @keyup.enter="saveTitle"
          @keyup.escape="cancelEdit"
        />
        <!-- 右侧区域：星标 + 时间 -->
        <span class="right-area" v-if="!props.compact">
          <!-- 星标优先级（可点击设置） -->
          <span class="priority-stars">
            <NIcon
              v-for="i in 3"
              :key="i"
              :component="i <= priorityStars ? StarIcon : StarOutlineIcon"
              size="14"
              :class="{ star: i <= priorityStars }"
              class="star-icon"
              @click.stop="handleSetPriority(i)"
            />
          </span>
          <!-- 时间区域 -->
          <span class="time-area">
            <!-- 开始时间 -->
            <NPopover trigger="click" placement="bottom" :show-arrow="false">
              <template #trigger>
                <span class="date-display">
                  <NIcon :component="CalendarIcon" size="12" />
                  <span>{{ formatDate(props.task.startDate) || '开始' }}</span>
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

            <!-- 截止时间 -->
            <NPopover trigger="click" placement="bottom" :show-arrow="false">
              <template #trigger>
                <span class="date-display">
                  <NIcon :component="ClockIcon" size="12" />
                  <span>{{ formatDate(props.task.dueDate) || '截止' }}</span>
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

            <!-- 倒计天数 -->
            <span v-if="countdownText" class="countdown" :class="countdownClass">
              {{ countdownText }}
            </span>
          </span>
        </span>
        <!-- 精简模式：只显示倒计时 -->
        <span v-if="props.compact && countdownText" class="compact-countdown" :class="countdownClass">
          {{ countdownText }}
        </span>
        <!-- 展开/折叠提示图标 -->
        <span v-if="needsExpand && !props.compact" class="expand-icon" :class="{ expanded: isExpanded }">
          <NIcon :component="ExpandIcon" size="14" />
        </span>
      </div>
      <!-- 描述（可点击编辑，支持多行） -->
      <div
        v-if="props.task.description || isEditingDesc"
        class="task-desc"
        :class="{ expanded: isExpanded }"
        v-show="!props.compact"
      >
        <div v-if="!isEditingDesc" class="desc-content" @click="startEditDesc">{{ props.task.description }}</div>
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
          @click.stop
        />
      </div>
      <!-- 添加描述提示（没有描述时显示） -->
      <div v-else class="task-desc-empty" @click="startEditDesc" v-show="!props.compact">
        点击添加描述...
      </div>
    </div>
  </div>

  <!-- 图片预览弹窗 -->
  <NModal v-model:show="showImagePreview" :mask-closable="true" @after-leave="closeImagePreview" @keydown.esc="closeImagePreview">
    <div class="image-preview-overlay" @click="closeImagePreview" @wheel="handleWheel">
      <div class="image-preview-container" @click.stop>
        <img
          :src="`data:image/jpeg;base64,${props.task.thumbnailBase64}`"
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
.simple-card {
  width: 100%;
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  transition: box-shadow 0.2s ease, border-color 0.2s ease;
  user-select: none;
  display: flex;
  align-items: flex-start;
  font-family: var(--task-font-family, inherit);
  gap: 12px;
}

html.dark .simple-card {
  background: #2a2a2a;
  border-color: #444;
}

.simple-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  border-top-color: #4A90D9;
  border-right-color: #4A90D9;
  border-bottom-color: #4A90D9;
  transform: scale(1.01);
}

html.dark .simple-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  border-top-color: #4A90D9;
  border-right-color: #4A90D9;
  border-bottom-color: #4A90D9;
  transform: scale(1.01);
}

.simple-card.done {
  opacity: 0.5;
}

/* 精简模式样式 */
.simple-card.compact {
  padding: 8px 10px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(74, 144, 217, 0.2);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}

html.dark .simple-card.compact {
  background: rgba(42, 42, 42, 0.95);
  border-color: rgba(91, 164, 245, 0.25);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
}

/* 精简模式禁用 hover 放大，避免被 overflow 裁切 */
.simple-card.compact:hover {
  transform: none;
}

.simple-card.compact .task-title {
  font-size: 18px;
  font-weight: 600;
  color: #1a1a1a;
}

html.dark .simple-card.compact .task-title {
  color: #f0f0f0;
}

.simple-card.compact .compact-countdown {
  padding: 3px 10px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 600;
  margin-left: 12px;
  flex-shrink: 0;
}

.simple-card.compact .compact-countdown.normal {
  background: #e8f5e9;
  color: #28C840;
}

.simple-card.compact .compact-countdown.soon {
  background: rgba(224, 82, 82, 0.12);
  color: #E05252;
}

.simple-card.compact .compact-countdown.urgent {
  background: #ffebee;
  color: #E05252;
  animation: pulse 1.5s ease-in-out infinite;
}

.simple-card.compact .compact-countdown.overdue {
  background: #f5f5f5;
  color: #999;
}

html.dark .simple-card.compact .compact-countdown.normal {
  background: rgba(40, 200, 64, 0.2);
  color: #4ade80;
}

html.dark .simple-card.compact .compact-countdown.soon {
  background: rgba(224, 82, 82, 0.25);
  color: #f87171;
}

html.dark .simple-card.compact .compact-countdown.urgent {
  background: rgba(224, 82, 82, 0.2);
  color: #f87171;
  animation: pulse 1.5s ease-in-out infinite;
}

html.dark .simple-card.compact .compact-countdown.overdue {
  background: rgba(100, 100, 100, 0.2);
  color: #888;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

/* 精简模式 - 优先级底色 */
.simple-card.compact.priority-high-bg {
  background: rgba(255, 248, 225, 0.95);
  border-color: rgba(255, 183, 77, 0.4);
}

html.dark .simple-card.compact.priority-high-bg {
  background: rgba(62, 39, 35, 0.95);
  border-color: rgba(255, 183, 77, 0.35);
}

.simple-card.compact.priority-medium-bg {
  background: rgba(227, 242, 253, 0.95);
  border-color: rgba(100, 181, 246, 0.4);
}

html.dark .simple-card.compact.priority-medium-bg {
  background: rgba(25, 40, 60, 0.95);
  border-color: rgba(100, 181, 246, 0.35);
}

.simple-card.compact.priority-low-bg {
  background: rgba(232, 245, 233, 0.95);
  border-color: rgba(102, 187, 106, 0.4);
}

html.dark .simple-card.compact.priority-low-bg {
  background: rgba(27, 45, 35, 0.95);
  border-color: rgba(102, 187, 106, 0.35);
}

.status-checkbox {
  flex-shrink: 0;
  margin-top: 2px;
}

.thumbnail-wrapper {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  overflow: hidden;
  background: #f0f0f0;
}

html.dark .thumbnail-wrapper {
  background: #333;
}

.thumbnail {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.task-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.task-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.task-title {
  font-size: var(--task-font-size, 14px);
  color: #333;
  font-weight: 500;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

html.dark .task-title {
  color: #e0e0e0;
}

.simple-card:hover .task-title {
  color: #FFB800;
}

html.dark .simple-card:hover .task-title {
  color: #FFB800;
}

.simple-card.done .task-title {
  text-decoration: line-through;
}

.title-input {
  font-size: var(--task-font-size, 14px);
  font-weight: 500;
  color: #333;
  background: transparent;
  border: none;
  outline: none;
  flex: 1;
  min-width: 100px;
  padding: 0;
  font-family: inherit;
}

html.dark .title-input {
  color: #e0e0e0;
}

.title-input::placeholder {
  color: #999;
}

html.dark .title-input::placeholder {
  color: #666;
}

.right-area {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  flex-shrink: 0;
}

.priority-stars {
  display: flex;
  gap: 2px;
  color: #ccc;
}

.priority-stars .star-icon {
  cursor: pointer;
  transition: color 0.15s ease, transform 0.15s ease;
}

.priority-stars .star-icon:hover {
  transform: scale(1.15);
}

.priority-stars .star {
  color: #FFB800;
}

.time-area {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: calc(var(--task-font-size, 14px) * 0.857);
}

.date-display {
  display: flex;
  align-items: center;
  gap: 3px;
  color: #888;
  cursor: pointer;
  padding: 1px 4px;
  border-radius: 4px;
  transition: background 0.15s ease;
}

html.dark .date-display {
  color: #999;
}

.date-display:hover {
  background: rgba(74, 144, 217, 0.1);
  color: #4A90D9;
}

html.dark .date-display:hover {
  background: rgba(74, 144, 217, 0.15);
}

.countdown {
  padding: 1px 6px;
  border-radius: 8px;
  font-size: calc(var(--task-font-size, 14px) * 0.786);
  font-weight: 500;
}

.countdown.normal {
  background: #e8f5e9;
  color: #28C840;
}

.countdown.soon {
  background: rgba(224, 82, 82, 0.1);
  color: #E05252;
}

.countdown.urgent {
  background: #ffebee;
  color: #E05252;
}

.countdown.overdue {
  background: #f5f5f5;
  color: #999;
}

html.dark .countdown.normal {
  background: rgba(40, 200, 64, 0.15);
  color: #28C840;
}

html.dark .countdown.soon {
  background: rgba(224, 82, 82, 0.2);
  color: #E05252;
}

html.dark .countdown.urgent {
  background: rgba(224, 82, 82, 0.15);
  color: #E05252;
}

html.dark .countdown.overdue {
  background: rgba(100, 100, 100, 0.15);
  color: #888;
}

.expand-icon {
  color: #4A90D9;
  flex-shrink: 0;
  transition: transform 0.3s ease;
  cursor: pointer;
  margin-top: 2px;
}

.expand-icon.expanded {
  transform: rotate(180deg);
}

.task-desc {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #888;
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-height: 18px;
  transition: max-height 0.3s ease, white-space 0.3s ease;
}

html.dark .task-desc {
  color: #999;
}

.task-desc.expanded {
  white-space: pre-wrap;
  max-height: calc(20 * 18px); /* 限制20行 */
  overflow-y: auto;
}

.desc-content {
  white-space: pre-wrap;
  word-break: break-word;
}

.task-desc-empty {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #999;
  margin-top: 4px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
  transition: background 0.15s ease;
}

html.dark .task-desc-empty {
  color: #666;
}

.task-desc-empty:hover {
  background: rgba(74, 144, 217, 0.1);
  color: #4A90D9;
}

html.dark .task-desc-empty:hover {
  background: rgba(74, 144, 217, 0.15);
}

.desc-input {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #888;
  background: transparent;
  border: none;
  outline: none;
  width: 100%;
  padding: 0;
  font-family: inherit;
  resize: none;
  line-height: 18px;
}

html.dark .desc-input {
  color: #999;
}

.desc-input::placeholder {
  color: #aaa;
}

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