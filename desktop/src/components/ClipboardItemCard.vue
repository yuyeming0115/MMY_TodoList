<script setup lang="ts">
import { NIcon, NDropdown, NInput, useMessage } from 'naive-ui';
import { h, ref, computed, nextTick, onMounted, onUnmounted } from 'vue';
import {
  TrashOutline as DeleteIcon, CopyOutline as CopyIcon, StarOutline as StarIcon, Star as StarFilledIcon,
  CreateOutline as EditIcon, TimeOutline as TimeIcon, CheckboxOutline as SelectIcon, FolderOutline as FolderIcon,
  ReorderTwoOutline as DragIcon, FolderOpenOutline as FolderOpenIcon,
} from '@vicons/ionicons5';
import type { ClipboardItem } from '../types';
import { useClipboardStore } from '../stores/clipboardStore';
import { BUILTIN_CLIPBOARD_CATEGORIES } from '../types';

const props = defineProps<{
  item: ClipboardItem;
  compact?: boolean;
  stacked?: boolean;
  showCheckbox?: boolean;
  selected?: boolean;
  selectionAnchor?: string | null;
}>();

const emit = defineEmits<{
  (e: 'delete', id: string): void;
  (e: 'update-priority', item: ClipboardItem, priority: 1 | 2 | 3): void;
  (e: 'contextmenu', event: MouseEvent, item: ClipboardItem): void;
  (e: 'toggle-select', id: string, shift: boolean): void;
  (e: 'enter-select-mode'): void;
  (e: 'move-to-category', item: ClipboardItem, categoryId: string): void;
}>();

const message = useMessage();
const clipboardStore = useClipboardStore();
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const isCrossDragging = ref(false);
const dragHandleRef = ref<HTMLElement | null>(null);

// 用原生 mousedown 拦截 Sortable.js 的事件捕获，让它无法调用 preventDefault() 阻断原生拖拽
function blockSortableMousedown(e: Event) {
  e.stopImmediatePropagation();
}

onMounted(() => {
  dragHandleRef.value?.addEventListener('mousedown', blockSortableMousedown, true);
});

onUnmounted(() => {
  dragHandleRef.value?.removeEventListener('mousedown', blockSortableMousedown, true);
});

const isFavorite = computed(() => props.item.categoryId === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE);
const isTextItem = computed(() => !props.item.imageBase64 && !props.item.imagePath);
const isBuiltinCategory = computed(() =>
  ([BUILTIN_CLIPBOARD_CATEGORIES.TEXT, BUILTIN_CLIPBOARD_CATEGORIES.IMAGE] as string[]).includes(props.item.categoryId)
);

// 过期时间相关
const expiryLabel = computed(() => {
  if (!props.item.expiresAt) return null;
  const now = Date.now();
  const diff = props.item.expiresAt - now;
  if (diff <= 0) return null;

  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}天后过期`;
  if (hours > 0) return `${hours}小时后过期`;
  const mins = Math.floor(diff / (1000 * 60));
  return `${mins}分钟后过期`;
});

const isExpiringSoon = computed(() => {
  if (!props.item.expiresAt) return false;
  const diff = props.item.expiresAt - Date.now();
  return diff > 0 && diff < 24 * 60 * 60 * 1000;
});

const displayContent = computed(() => {
  if (!props.compact) return props.item.content;
  return props.item.content.length > 50
    ? props.item.content.substring(0, 50) + '…'
    : props.item.content;
});

// 编辑模式
const isEditing = ref(false);
const editTitle = ref('');
const editContent = ref('');
const editTextareaRef = ref<any>(null);

const categoryOptions = computed(() => {
  const allCats = clipboardStore.categories;
  return allCats
    .filter(cat => cat.id !== props.item.categoryId)
    .map(cat => ({
      label: cat.name,
      key: `move_${cat.id}`,
      icon: () => h(NIcon, {
        component: FolderIcon,
        size: 16,
        style: { color: cat.color },
      }),
    }));
});

const contextMenuOptions = computed(() => {
  const options: any[] = [
    { label: '复制', key: 'copy', icon: () => h(NIcon, { component: CopyIcon, size: 16 }) },
    { label: isFavorite.value ? '取消收藏' : '收藏', key: 'favorite', icon: () => h(NIcon, { component: isFavorite.value ? StarFilledIcon : StarIcon, size: 16, style: { color: isFavorite.value ? '#F39C12' : '#333' } }) },
  ];

  // 只有文本类型才显示编辑
  if (isTextItem.value) {
    options.push({ label: '编辑', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 16 }) });
  }

  // 图片类型且有本地路径，显示"打开图片所在文件夹"
  if (props.item.imagePath) {
    options.push({ label: '打开图片所在文件夹', key: 'openFolder', icon: () => h(NIcon, { component: FolderOpenIcon, size: 16 }) });
  }

  // 内置分类（文本/图像）且非收藏，显示设置过期时间
  if (isBuiltinCategory.value && !isFavorite.value) {
    options.push({
      key: 'expiry',
      label: '设置过期时间',
      icon: () => h(NIcon, { component: TimeIcon, size: 16 }),
      children: [
        { label: '1小时', key: 'expiry_1h' },
        { label: '1天', key: 'expiry_1d' },
        { label: '7天', key: 'expiry_7d' },
        { label: '30天', key: 'expiry_30d' },
        { label: '永不过期', key: 'expiry_never' },
      ],
    });
  }

  // 移动到分类（有自定义分类时显示）
  if (categoryOptions.value.length > 0) {
    options.push({ type: 'divider', key: 'd2' });
    options.push({
      key: 'move',
      label: '移动到分类',
      icon: () => h(NIcon, { component: FolderIcon, size: 16 }),
      children: categoryOptions.value,
    });
  }

  options.push({ type: 'divider', key: 'd1' });
  options.push({ label: '进入选择模式', key: 'enter_select', icon: () => h(NIcon, { component: SelectIcon, size: 16 }) });
  options.push({ label: '删除', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } }) });
  return options;
});

async function copyContent() {
  try {
    // 如果有 imagePath，从文件读取原图
    if (props.item.imagePath) {
      const { invoke } = await import('@tauri-apps/api/core');

      try {
        const base64 = await invoke<string>('read_clipboard_image_file', { path: props.item.imagePath });
        const base64Data = base64.replace(/^data:image\/\w+;base64,/, '');
        await invoke('write_image_to_clipboard', { base64: base64Data });
        message.success('已复制图片');
      } catch (e) {
        // 文件读取失败，说明文件已被删除
        console.error('图片文件读取失败:', e);
        message.warning('图片文件已被删除，卡片已自动清理');
        emit('delete', props.item.id);
        return;
      }
    } else if (props.item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success('已复制图片');
    } else if (props.item.content) {
      await navigator.clipboard.writeText(props.item.content);
      message.success('已复制');
    }
  } catch (e) {
    // 其他复制失败情况
    if (props.item.content) {
      const ta = document.createElement('textarea');
      ta.value = props.item.content;
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      if (ok) {
        message.success('已复制');
        return;
      }
    }
    message.error('复制失败');
    console.error('复制失败:', e);
  }
}

async function handleFavorite() {
  const result = await clipboardStore.favoriteItem(props.item);
  if (result === 'favorited') {
    message.success('已收藏');
  } else if (result === 'unfavorited') {
    message.success('已取消收藏');
  } else {
    message.error('收藏分类不存在，请刷新后重试');
  }
}

function handleClick(e: MouseEvent) {
  if (props.showCheckbox || e.ctrlKey || e.metaKey || e.shiftKey) {
    emit('toggle-select', props.item.id, e.shiftKey);
    return;
  }
  copyContent();
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  showContextMenu.value = true;
}

function setExpiry(key: string) {
  const now = Date.now();
  let expiresAt: number | null = null;
  switch (key) {
    case 'expiry_1h': expiresAt = now + 1 * 60 * 60 * 1000; break;
    case 'expiry_1d': expiresAt = now + 1 * 24 * 60 * 60 * 1000; break;
    case 'expiry_7d': expiresAt = now + 7 * 24 * 60 * 60 * 1000; break;
    case 'expiry_30d': expiresAt = now + 30 * 24 * 60 * 60 * 1000; break;
    case 'expiry_never': expiresAt = null; break;
  }
  clipboardStore.setItemExpiry(props.item.id, expiresAt);
  const label = { expiry_1h: '1小时', expiry_1d: '1天', expiry_7d: '7天', expiry_30d: '30天', expiry_never: '永不过期' }[key];
  message.success(`已设置：${label}`);
}

function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'copy') copyContent();
  if (key === 'favorite') handleFavorite();
  if (key === 'edit') startEdit();
  if (key === 'openFolder') openImageFolder();
  if (key === 'delete') emit('delete', props.item.id);
  if (key === 'enter_select') emit('enter-select-mode');
  if (key.startsWith('move_')) {
    const catId = key.slice(5);
    emit('move-to-category', props.item, catId);
    message.success('已移动');
  }
  if (key.startsWith('expiry_')) setExpiry(key);
}

async function startEdit() {
  isEditing.value = true;
  editTitle.value = props.item.title;
  editContent.value = props.item.content;
  await nextTick();
  editTextareaRef.value?.focus();
}

// 打开图片所在文件夹
async function openImageFolder() {
  if (!props.item.imagePath) return;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('reveal_file_in_folder', { path: props.item.imagePath });
    message.success('已打开文件夹');
  } catch (e) {
    console.error('打开文件夹失败:', e);
    message.error('打开文件夹失败');
  }
}

function saveEdit() {
  if (!editTitle.value.trim() && !editContent.value.trim()) {
    // 如果都为空，取消编辑
    cancelEdit();
    return;
  }
  const updated: ClipboardItem = {
    ...props.item,
    title: editTitle.value.trim() || props.item.title,
    content: editContent.value,
  };
  clipboardStore.updateItem(updated);
  isEditing.value = false;
  message.success('已保存');
}

function cancelEdit() {
  isEditing.value = false;
  editTitle.value = '';
  editContent.value = '';
}

// 跨应用拖拽处理
async function handleCrossAppDragStart(e: DragEvent) {
  if (!e.dataTransfer) return;

  e.stopPropagation();
  isCrossDragging.value = true;
  e.dataTransfer.effectAllowed = 'copy';

  if (props.item.imagePath) {
    // 有本地文件路径：用文件 URI 实现真正拖拽到外部应用
    const filePath = props.item.imagePath.replace(/\\/g, '/');
    e.dataTransfer.setData('text/uri-list', `file:///${filePath}`);
    e.dataTransfer.setData('text/plain', props.item.imagePath);
  } else if (props.item.imageBase64) {
    // 无本地路径（仅 base64）：保持剪贴板方案
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.info('图片已复制到剪贴板，请在目标窗口 Ctrl+V 粘贴');
    } catch {
      message.error('图片复制失败');
    }
  } else {
    // 文本
    e.dataTransfer.setData('text/plain', props.item.content);
  }
}

function handleCrossAppDragEnd(_e: DragEvent) {
  isCrossDragging.value = false;
}
</script>

<template>
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
  <div class="task-card" :class="{ compact: props.compact, expiring: isExpiringSoon, stacked: props.stacked, selected: props.selected }" @click="handleClick" @contextmenu="handleContextMenu">
    <label v-if="props.showCheckbox" class="checkbox-overlay" @click.stop="emit('toggle-select', props.item.id, false)">
      <input type="checkbox" :checked="props.selected" />
      <span class="checkmark"></span>
    </label>
    <div
      class="cross-app-drag-handle"
      draggable="true"
      ref="dragHandleRef"
      @dragstart="handleCrossAppDragStart"
      @dragend="handleCrossAppDragEnd"
      :class="{ 'is-dragging': isCrossDragging }"
      title="拖拽到其它应用"
    >
      <NIcon :component="DragIcon" size="18" />
    </div>
    <div class="task-content">
      <!-- 纯图片：显示缩略图（优先用 thumbnailBase64，回退到 imageBase64） -->
      <div v-if="item.imagePath || item.imageBase64" class="image-only">
        <div class="task-thumbnail" :class="{ 'compact-thumb': props.compact }">
          <img :src="item.thumbnailBase64 || item.imageBase64" alt="剪贴板图片" />
        </div>
      </div>

      <!-- 文本：标题 + 内容 -->
      <div v-if="!item.imagePath && !item.imageBase64">
        <!-- 编辑模式 -->
        <div v-if="isEditing" class="edit-mode" @click.stop>
          <NInput
            v-model:value="editTitle"
            size="small"
            placeholder="标题"
            class="edit-title-input"
            @keyup.enter="saveEdit"
          />
          <NInput
            ref="editTextareaRef"
            v-model:value="editContent"
            type="textarea"
            size="small"
            placeholder="内容"
            :autosize="{ minRows: 2, maxRows: 6 }"
            class="edit-content-input"
          />
          <div class="edit-actions">
            <button class="edit-btn save" @click="saveEdit">保存</button>
            <button class="edit-btn cancel" @click="cancelEdit">取消</button>
          </div>
        </div>
        <!-- 正常显示 -->
        <template v-else>
          <div v-if="item.content" class="task-desc">{{ displayContent }}</div>
        </template>
      </div>
    </div>

    <!-- 过期时间提示 -->
    <div v-if="expiryLabel" class="expiry-badge" :class="{ warning: isExpiringSoon }">
      <NIcon :component="TimeIcon" size="10" />
      {{ expiryLabel }}
    </div>
  </div>
</template>

<style scoped>
.task-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  border-left: 3px solid #4A90D9;
  font-size: var(--task-font-size, 14px);
  font-family: var(--task-font-family, inherit);
  transition: box-shadow 0.2s, border-color 0.2s, transform 0.2s;
  cursor: pointer;
  transform-origin: center center;
}

.task-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  border-color: #4A90D9;
  transform: scale(1.01);
}

html.dark .task-card {
  background: #2a2a2a;
  border-color: #444;
}

html.dark .task-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  border-color: #4A90D9;
  transform: scale(1.01);
}

.task-card:active {
  transform: scale(0.99);
}

/* 即将过期的卡片样式 */
.task-card.expiring {
  opacity: 0.7;
}

.task-card.expiring:hover {
  opacity: 1;
}

/* 层叠模式卡片样式 */
.task-card.stacked {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
}

.task-card.stacked:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
}

html.dark .task-card.stacked {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

html.dark .task-card.stacked:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}

/* 精简模式样式 */
.task-card.compact {
  padding: 8px 10px;
  border-radius: 10px;
  border-left-width: 3px;
}

/* 精简模式禁用 hover 放大，避免被 overflow 裁切 */
.task-card.compact:hover {
  transform: none;
}

.task-card.compact .task-title {
  font-size: 15px;
  font-weight: 600;
  color: #1a1a1a;
}

html.dark .task-card.compact .task-title {
  color: #f0f0f0;
}

.task-card.compact .task-desc {
  font-size: 12px;
  line-height: 1.4;
  max-height: 36px;
  -webkit-line-clamp: 2;
}

.task-card.compact .task-thumbnail.compact-thumb {
  max-width: 120px;
}

.task-card.compact .task-thumbnail.compact-thumb img {
  max-height: 80px;
  object-fit: cover;
}

.task-content {
  min-width: 0;
}

.task-title {
  font-size: var(--task-font-size, 14px);
  font-weight: 500;
  color: #333;
  word-break: break-all;
}

.task-card:hover .task-title {
  color: #FFB800;
}

html.dark .task-card:hover .task-title {
  color: #FFB800;
}

html.dark .task-title {
  color: #e0e0e0;
}

.task-desc {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #888;
  margin-top: 4px;
  word-break: break-all;
  white-space: pre-wrap;
  max-height: 60px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  transition: color 0.15s;
}

.task-card:hover .task-desc {
  color: #FFB800;
}

html.dark .task-desc {
  color: #999;
}

html.dark .task-card:hover .task-desc {
  color: #FFB800;
}

.image-only {
  width: 100%;
}

.task-thumbnail {
  max-width: 200px;
  border-radius: 8px;
  overflow: hidden;
}

.task-thumbnail img {
  width: 100%;
  display: block;
}

/* 编辑模式 */
.edit-mode {
  width: 100%;
}

.edit-title-input {
  margin-bottom: 8px;
}

.edit-content-input {
  margin-bottom: 8px;
}

.edit-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.edit-btn {
  padding: 4px 16px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.edit-btn.save {
  background: #4A90D9;
  color: #fff;
}

.edit-btn.save:hover {
  background: #3A7BC8;
}

.edit-btn.cancel {
  background: transparent;
  color: #999;
  border: 1px solid #ddd;
}

.edit-btn.cancel:hover {
  background: rgba(0, 0, 0, 0.05);
}

html.dark .edit-btn.cancel {
  border-color: #444;
  color: #888;
}

html.dark .edit-btn.cancel:hover {
  background: rgba(255, 255, 255, 0.05);
}

/* 过期时间徽章 */
.expiry-badge {
  position: absolute;
  bottom: 4px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: 9px;
  color: #999;
  opacity: 0.7;
  pointer-events: none;
}

.expiry-badge.warning {
  color: #E05252;
  opacity: 1;
}

html.dark .expiry-badge {
  color: #777;
}

html.dark .expiry-badge.warning {
  color: #E05252;
}

.task-card {
  position: relative;
}

/* 复选框覆盖层 */
.checkbox-overlay {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.checkbox-overlay input[type="checkbox"] {
  display: none;
}

.checkmark {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: 2px solid #ccc;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.9);
  transition: all 0.15s;
  pointer-events: none;
}

html.dark .checkmark {
  background: rgba(42, 42, 42, 0.9);
  border-color: #555;
}

.checkmark::after {
  content: '';
  width: 6px;
  height: 10px;
  border: solid #fff;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg) scale(0);
  transition: transform 0.15s;
}

.checkbox-overlay input:checked + .checkmark {
  background: #4A90D9;
  border-color: #4A90D9;
}

.checkbox-overlay input:checked + .checkmark::after {
  transform: rotate(45deg) scale(1);
}

/* 选中状态卡片 */
.task-card.selected {
  border-left-color: #4A90D9;
  background: rgba(74, 144, 217, 0.06);
}

html.dark .task-card.selected {
  background: rgba(74, 144, 217, 0.1);
}

/* 跨应用拖拽手柄 */
.cross-app-drag-handle {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  cursor: grab;
  opacity: 0;
  transition: opacity 0.15s, background 0.15s;
  z-index: 5;
  color: #999;
}

.task-card:hover .cross-app-drag-handle {
  opacity: 1;
}

.cross-app-drag-handle:hover {
  background: rgba(0, 0, 0, 0.08);
}

html.dark .cross-app-drag-handle:hover {
  background: rgba(255, 255, 255, 0.1);
}

.cross-app-drag-handle:active {
  cursor: grabbing;
}

.cross-app-drag-handle.is-dragging {
  opacity: 1;
  background: rgba(74, 144, 217, 0.2);
  color: #4A90D9;
}
</style>
