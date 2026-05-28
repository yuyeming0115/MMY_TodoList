<script setup lang="ts">
import { NIcon, NDropdown, NInput, useMessage } from 'naive-ui';
import { h, ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue';
import {
  TrashOutline as DeleteIcon,
  CreateOutline as EditIcon, TimeOutline as TimeIcon, CheckboxOutline as SelectIcon, FolderOutline as FolderIcon,
  ReorderTwoOutline as DragIcon, FolderOpenOutline as FolderOpenIcon, LockClosedOutline as LockIcon,
  LockOpenOutline as UnlockIcon,
  ArrowUpOutline as TopIcon,
} from '@vicons/ionicons5';
import type { ClipboardItem } from '../types';
import { useClipboardStore } from '../stores/clipboardStore';
import { useImageCacheStore } from '../stores/imageCacheStore';
import { useI18n } from '../composables/useI18n';

const props = defineProps<{
  item: ClipboardItem;
  compact?: boolean;
  stacked?: boolean;
  showCheckbox?: boolean;
  selected?: boolean;
  selectionAnchor?: string | null;
  isVisible?: boolean; // 新增：是否在可视区域
}>();

const emit = defineEmits<{
  (e: 'delete', id: string): void;
  (e: 'update-priority', item: ClipboardItem, priority: 1 | 2 | 3): void;
  (e: 'contextmenu', event: MouseEvent, item: ClipboardItem): void;
  (e: 'toggle-select', id: string, shift: boolean): void;
  (e: 'enter-select-mode'): void;
  (e: 'move-to-category', item: ClipboardItem, categoryId: string): void;
  (e: 'batch-move-to-category', categoryId: string): void;
  (e: 'batch-lock'): void;
  (e: 'batch-delete'): void;
  (e: 'move-to-top', item: ClipboardItem): void;
  (e: 'editing-change', isEditing: boolean): void;
}>();

const message = useMessage();
const clipboardStore = useClipboardStore();
const imageCacheStore = useImageCacheStore();
const { t } = useI18n();
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuPlacement = ref<'bottom-start' | 'top-start'>('bottom-start');
const isCrossDragging = ref(false);
const dragHandleRef = ref<HTMLElement | null>(null);

// 图片懒加载：只有可视时才解码
const shouldLoadImage = ref(false);
const imageSrc = ref<string>('');

// 使用两级缓存加载图片（学习 Ditto 架构）
function loadImageWithCache() {
  // 1. 先检查缓存
  const cached = imageCacheStore.getCachedImage(props.item.id);
  if (cached) {
    imageSrc.value = cached;
    return;
  }

  // 2. 缓存不存在，检查是否有图片路径需要加载
  if (props.item.imagePath && imageCacheStore.needsLoad(props.item.id)) {
    // 加入异步加载队列，不阻塞渲染
    imageCacheStore.addToLoadQueue(props.item.id, props.item.imagePath);
  }

  // 3. 如果有 imageBase64 备用，直接使用
  if (props.item.imageBase64) {
    imageSrc.value = props.item.imageBase64;
    // 同时缓存
    imageCacheStore.cacheImage(props.item.id, props.item.imageBase64);
  }
}

watch(() => props.isVisible, (visible) => {
  if (visible && (props.item.imagePath || props.item.imageBase64) && !shouldLoadImage.value) {
    shouldLoadImage.value = true;
    loadImageWithCache();
  }
}, { immediate: true });

// 监听缓存更新：当后台加载完成后，更新显示
watch(() => imageCacheStore.getCachedImage(props.item.id), (cached) => {
  if (cached && shouldLoadImage.value && !imageSrc.value) {
    imageSrc.value = cached;
  }
});

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

const isLocked = computed(() => clipboardStore.isItemLocked(props.item));
const isTextItem = computed(() => !props.item.imageBase64 && !props.item.imagePath);

// 过期时间相关
const expiryLabel = computed(() => {
  if (!props.item.expiresAt) return null;
  const now = Date.now();
  const diff = props.item.expiresAt - now;
  if (diff <= 0) return null;

  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(hours / 24);

  let timeStr: string;
  if (days > 0) {
    timeStr = `${days}${t('expiry.daysLater')}`;
  } else if (hours > 0) {
    timeStr = `${hours}${t('expiry.hoursLater')}`;
  } else {
    const mins = Math.floor(diff / (1000 * 60));
    timeStr = `${mins}${t('expiry.minutesLater')}`;
  }

  return t('expiry.expiresIn', { time: timeStr });
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
  const options: any[] = [];

  // 图片类型且有本地路径，显示"打开图片所在文件夹"
  if (props.item.imagePath) {
    options.push({ label: t('contextMenu.openFolder'), key: 'openFolder', icon: () => h(NIcon, { component: FolderOpenIcon, size: 16 }) });
  }

  // 移动到分类（直接显示分类列表）
  if (categoryOptions.value.length > 0) {
    if (options.length > 0) options.push({ type: 'divider', key: 'd2' });
    options.push(...categoryOptions.value);
  }

  // 选择模式下显示"清空所有未锁定项"
  if (props.showCheckbox) {
    if (options.length > 0) options.push({ type: 'divider', key: 'd3' });
    options.push({ label: t('contextMenu.clearAllUnlocked'), key: 'clear_all_unlocked', icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } }) });
  }
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
        message.success(t('messages.imageCopied'));
      } catch (e) {
        // 文件读取失败，说明文件已被删除
        console.error('图片文件读取失败:', e);
        message.warning(t('messages.imageNotFound'));
        emit('delete', props.item.id);
        return;
      }
    } else if (props.item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success(t('messages.imageCopied'));
    } else if (props.item.content) {
      await navigator.clipboard.writeText(props.item.content);
      message.success(t('messages.copied'));
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
        message.success(t('messages.copied'));
        return;
      }
    }
    message.error(t('messages.copyFailed'));
    console.error('复制失败:', e);
  }
}

function handleClick(e: MouseEvent) {
  // 选择模式下点击触发选中
  if (props.showCheckbox || e.ctrlKey || e.metaKey || e.shiftKey) {
    emit('toggle-select', props.item.id, e.shiftKey);
    return;
  }
  copyContent();
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault();

  // 获取窗口尺寸和预估菜单尺寸
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  // 预估菜单宽度（根据选项数量，通常约 200px）
  const estimatedMenuWidth = 220;
  // 预估菜单高度（每个选项约 36px，分隔线约 10px）
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
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && showContextMenu.value) {
    showContextMenu.value = false;
  }
}

onMounted(() => {
  dragHandleRef.value?.addEventListener('mousedown', blockSortableMousedown, true);
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  dragHandleRef.value?.removeEventListener('mousedown', blockSortableMousedown, true);
  window.removeEventListener('keydown', handleKeydown);
});

async function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'clear_all_unlocked') {
    const count = await clipboardStore.clearAllUnlocked();
    message.success(t('messages.clearAllUnlockedDone', { count }));
    return;
  }
  if (key === 'openFolder') openImageFolder();
  if (key.startsWith('move_')) {
    const catId = key.slice(5);
    // 选择模式下批量移动
    if (props.showCheckbox) {
      emit('batch-move-to-category', catId);
    } else {
      emit('move-to-category', props.item, catId);
      message.success(t('messages.moved'));
    }
  }
}

// 微缩按钮操作
async function handleToggleLock(e: MouseEvent) {
  e.stopPropagation();
  if (props.showCheckbox) {
    emit('batch-lock');
  } else {
    const result = await clipboardStore.toggleItemLock(props.item);
    message.success(result === 'locked' ? t('messages.locked') : t('messages.unlocked'));
  }
}

function handleMoveToTop(e: MouseEvent) {
  e.stopPropagation();
  emit('move-to-top', props.item);
  message.success(t('messages.moved'));
}

function handleStartEdit(e: MouseEvent) {
  e.stopPropagation();
  startEdit();
}

function handleEnterSelectMode(e: MouseEvent) {
  e.stopPropagation();
  emit('enter-select-mode');
}

function handleDelete(e: MouseEvent) {
  e.stopPropagation();
  if (props.showCheckbox) {
    emit('batch-delete');
  } else {
    emit('delete', props.item.id);
  }
}

async function startEdit() {
  isEditing.value = true;
  emit('editing-change', true);
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
    message.success(t('messages.folderOpened'));
  } catch (e) {
    console.error('打开文件夹失败:', e);
    message.error(t('messages.openFolderFailed'));
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
  emit('editing-change', false);
  message.success(t('messages.saved'));
}

function cancelEdit() {
  isEditing.value = false;
  editTitle.value = '';
  editContent.value = '';
  emit('editing-change', false);
}

// 跨应用拖拽处理：写入剪贴板 + text/uri-list，拖拽结束自动 Ctrl+V
async function handleCrossAppDragStart(e: DragEvent) {
  if (!e.dataTransfer) return;

  e.stopPropagation();
  isCrossDragging.value = true;
  e.dataTransfer.effectAllowed = 'copy';

  if (props.item.imagePath || props.item.imageBase64) {
    const { invoke } = await import('@tauri-apps/api/core');
    // 先标记跳过下一次剪贴板监控，防止自我吞噬生成重复卡片
    await invoke('mark_clipboard_skip_next').catch(() => {});

    if (props.item.imagePath) {
      // 有本地文件：写 PNG + CF_HDROP（PS 需要文件拖拽）
      await invoke('get_image_for_drag', { id: props.item.id }).catch(() => {});
      const filePath = props.item.imagePath.replace(/\\/g, '/');
      e.dataTransfer.setData('text/uri-list', `file:///${filePath}`);
    } else if (props.item.imageBase64) {
      // 仅 base64：写 PNG
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data }).catch(() => {});
    }
  } else {
    // 文本
    e.dataTransfer.setData('text/plain', props.item.content);
  }
}

// 拖拽结束：图片类型统一自动 Ctrl+V 粘贴
async function handleCrossAppDragEnd(e: DragEvent) {
  isCrossDragging.value = false;

  if (
    (props.item.imagePath || props.item.imageBase64) &&
    e.dataTransfer?.dropEffect &&
    e.dataTransfer.dropEffect !== 'none'
  ) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      await invoke('simulate_ctrl_v');
    } catch {
      // 自动粘贴失败，静默（目标应用可能已通过 text/uri-list 接受了文件拖拽）
    }
  }
}
</script>

<template>
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
  <div class="task-card" :class="{ compact: props.compact, expiring: isExpiringSoon, stacked: props.stacked, selected: props.selected, selectMode: props.showCheckbox }" @click="handleClick" @contextmenu="handleContextMenu">
    <label v-if="props.showCheckbox" class="checkbox-overlay" @click.stop="emit('toggle-select', props.item.id, false)">
      <input type="checkbox" :checked="props.selected" />
      <span class="checkmark"></span>
    </label>

    <!-- 微缩按钮区域 - 精简模式下不渲染 -->
    <div v-if="!props.compact" class="action-buttons">
      <button class="action-btn" :class="{ active: isLocked }" :title="isLocked ? t('contextMenu.unlock') : t('contextMenu.lock')" @click="handleToggleLock">
        <NIcon :component="isLocked ? LockIcon : UnlockIcon" size="14" />
      </button>
      <button class="action-btn" :title="t('contextMenu.moveToTop')" @click="handleMoveToTop">
        <NIcon :component="TopIcon" size="14" />
      </button>
      <button v-if="isTextItem" class="action-btn" :title="t('contextMenu.edit')" @click="handleStartEdit">
        <NIcon :component="EditIcon" size="14" />
      </button>
      <button class="action-btn" :title="t('contextMenu.enterSelectMode')" @click="handleEnterSelectMode">
        <NIcon :component="SelectIcon" size="14" />
      </button>
      <button class="action-btn delete" :title="t('contextMenu.delete')" @click="handleDelete">
        <NIcon :component="DeleteIcon" size="14" />
      </button>
    </div>

    <!-- 跨应用拖拽手柄 - 精简模式下不渲染 -->
    <div
      v-if="!props.compact"
      class="cross-app-drag-handle"
      draggable="true"
      ref="dragHandleRef"
      @dragstart="handleCrossAppDragStart"
      @dragend="handleCrossAppDragEnd"
      :class="{ 'is-dragging': isCrossDragging }"
      :title="t('drag.crossAppDrag')"
    >
      <NIcon :component="DragIcon" size="18" />
    </div>
    <div class="task-content">
      <!-- 纯图片：显示缩略图（懒加载，只有可视时才解码） -->
      <div v-if="item.imagePath || item.imageBase64" class="image-only">
        <div class="task-thumbnail" :class="{ 'compact-thumb': props.compact }">
          <img v-if="shouldLoadImage" :src="imageSrc" alt="剪贴板图片" />
          <div v-else class="image-placeholder"></div>
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
            <button class="edit-btn save" @click="saveEdit">{{ t('messages.save') }}</button>
            <button class="edit-btn cancel" @click="cancelEdit">{{ t('messages.cancel') }}</button>
          </div>
        </div>
        <!-- 正常显示 -->
        <template v-else>
          <div v-if="item.content" class="task-desc">{{ displayContent }}</div>
        </template>
      </div>
    </div>

    <!-- 过期时间提示 - 精简模式下不渲染 -->
    <div v-if="expiryLabel && !props.compact" class="expiry-badge" :class="{ warning: isExpiringSoon }">
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
  max-height: 120px;
  border-radius: 8px;
  overflow: hidden;
}

.task-thumbnail img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

/* 图片懒加载占位符 */
.image-placeholder {
  width: 100%;
  height: 100%;
  min-height: 80px;
  background: linear-gradient(135deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 200%;
  animation: placeholder-shimmer 1.5s infinite;
}

html.dark .image-placeholder {
  background: linear-gradient(135deg, #333 25%, #444 50%, #333 75%);
  background-size: 200% 200%;
}

@keyframes placeholder-shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
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
  top: 50%;
  left: 6px;
  transform: translateY(-50%);
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

/* 选择模式下卡片左侧留出空间 */
.task-card.selectMode {
  padding-left: 32px;
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

/* 微缩按钮区域 - 顶部右侧 */
.action-buttons {
  position: absolute;
  top: 4px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
  z-index: 10;
}

.task-card:hover .action-buttons {
  opacity: 1;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #999;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.action-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: #333;
}

html.dark .action-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #e0e0e0;
}

.action-btn.active {
  color: #4A90D9;
}

.action-btn.delete {
  color: #E05252;
}

.action-btn.delete:hover {
  background: rgba(224, 82, 82, 0.1);
}

/* 跨应用拖拽手柄 - 底部右侧（避免和微缩按钮冲突） */
.cross-app-drag-handle {
  position: absolute;
  bottom: 4px;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
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
