<script setup lang="ts">
import { ref, computed, watch, h, onMounted, onUnmounted } from 'vue';
import { NDropdown, NIcon, NInput, useMessage } from 'naive-ui';
import { TrashOutline as DeleteIcon, CopyOutline as CopyIcon, CreateOutline as EditIcon } from '@vicons/ionicons5';
import draggable from 'vuedraggable';
import { useClipboardStore } from '../stores/clipboardStore';
import { useI18n } from '../composables/useI18n';
import ClipboardItemCard from './ClipboardItemCard.vue';
import type { ClipboardItem } from '../types';

const clipboardStore = useClipboardStore();
const message = useMessage();
const { t } = useI18n();

const props = defineProps<{
  compact?: boolean;
  stacked?: boolean;
  stackGap?: number;
}>();

// 计算样式（确保响应式更新）
const stackStyle = computed(() => {
  if (props.stacked) {
    const gap = props.stackGap ?? 64;
    return { '--stack-gap': `${gap}px` };
  }
  return {};
});

const isDragging = ref(false);
const dragList = ref<ClipboardItem[]>([]);

// 批量选择
const selectMode = ref(false);
const selectedIds = ref(new Set<string>());
const selectionAnchor = ref<string | null>(null); // Shift 连选的锚点

// 切换单个选中（允许选中收藏卡用于移动操作）
function toggleSelect(id: string, isShift: boolean) {
  if (isShift && selectionAnchor.value) {
    // Shift 连选：选中锚点到当前之间的所有项目
    const ids = filteredItems.value.map(i => i.id);
    const anchorIdx = ids.indexOf(selectionAnchor.value);
    const currIdx = ids.indexOf(id);
    if (anchorIdx >= 0 && currIdx >= 0) {
      const start = Math.min(anchorIdx, currIdx);
      const end = Math.max(anchorIdx, currIdx);
      for (let i = start; i <= end; i++) {
        selectedIds.value.add(ids[i]);
      }
    }
  } else {
    if (selectedIds.value.has(id)) {
      selectedIds.value.delete(id);
    } else {
      selectedIds.value.add(id);
      selectionAnchor.value = id;
    }
  }
}

// 进入选择模式（从卡片菜单触发）
function enterSelectModeFromCard() {
  if (!selectMode.value) {
    selectMode.value = true;
  }
}

// 快捷键处理：A全选，ESC第1次清空，ESC第2次退出
function handleKeydown(e: KeyboardEvent) {
  // A键全选（在选择模式下）
  if ((e.key === 'a' || e.key === 'A') && (selectMode.value || selectedIds.value.size > 0)) {
    selectAll();
    return;
  }

  // ESC键处理
  if (e.key === 'Escape') {
    if (selectedIds.value.size > 0) {
      // 第1次ESC：清空选中
      selectedIds.value = new Set();
      selectionAnchor.value = null;
    } else if (selectMode.value) {
      // 第2次ESC（已清空状态）：退出选择模式
      selectMode.value = false;
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});

// 移动分类
function moveToCategory(item: ClipboardItem, categoryId: string) {
  item.categoryId = categoryId;
  clipboardStore.updateItem(item);
}

// 批量移动分类
async function batchMoveToCategory(categoryId: string) {
  const ids = [...selectedIds.value];
  if (ids.length === 0) return;

  let movedCount = 0;
  for (const id of ids) {
    const item = clipboardStore.items.find(i => i.id === id);
    if (item) {
      item.categoryId = categoryId;
      await clipboardStore.updateItem(item);
      movedCount++;
    }
  }

  if (movedCount > 0) {
    message.success(t('messages.movedSelected', { count: movedCount }));
  }
  selectedIds.value = new Set();
  selectMode.value = false;
  selectionAnchor.value = null;
}

// 全选
function selectAll() {
  const allIds = filteredItems.value.map(i => i.id);
  selectedIds.value = new Set(allIds);
  if (!selectMode.value) selectMode.value = true;
}

// 批量删除（过滤锁定卡片，包括卡片级别和分类级别锁定）
async function deleteSelected() {
  // 过滤出非锁定的选中项（同时检查卡片级别和分类级别）
  const ids = [...selectedIds.value].filter(id => {
    const item = clipboardStore.items.find(i => i.id === id);
    return item && !clipboardStore.isItemLocked(item);
  });

  if (ids.length === 0) {
    // 如果只有锁定卡片被选中，提示用户
    if (selectedIds.value.size > 0) {
      message.warning(t('messages.lockedCannotDelete'));
    }
    return;
  }

  await clipboardStore.removeItems(ids);
  message.success(t('messages.deleteSelected', { count: ids.length }));

  // 如果有锁定卡片被排除，额外提示
  const excludedCount = selectedIds.value.size - ids.length;
  if (excludedCount > 0) {
    message.info(t('messages.lockedExcludedFromDelete', { count: excludedCount }));
  }

  selectedIds.value = new Set();
  selectMode.value = false;
  selectionAnchor.value = null;
}

// 批量锁定
async function batchLock() {
  const ids = [...selectedIds.value];
  if (ids.length === 0) return;

  let lockedCount = 0;
  for (const id of ids) {
    const item = clipboardStore.items.find(i => i.id === id);
    if (item && !item.locked) {
      await clipboardStore.lockItem(item);
      lockedCount++;
    }
  }

  if (lockedCount > 0) {
    message.success(t('messages.lockedSelected', { count: lockedCount }));
  }
  selectedIds.value = new Set();
  selectMode.value = false;
  selectionAnchor.value = null;
}

// 过滤后的项目（精简模式和正常模式都使用 store 的过滤）
const filteredItems = computed(() => {
  // 精简模式和正常模式都使用 store 的 filteredItems（包含分类和搜索）
  let items = clipboardStore.filteredItems;

  // 精简模式下根据排序模式排序（正常模式已排序）
  if (props.compact) {
    const mode = clipboardStore.sortMode;
    if (mode === 'name') {
      items = [...items].sort((a, b) => a.title.localeCompare(b.title, 'zh'));
    } else if (mode === 'createdAt') {
      items = [...items].sort((a, b) => b.createdAt - a.createdAt);
    } else {
      items = [...items].sort((a, b) => a.sortOrder - b.sortOrder);
    }
  }

  return items;
});

// 拖拽是否启用（仅在自定义排序模式下）
const dragEnabled = computed(() => clipboardStore.sortMode === 'custom');

// 同步到拖拽列表
watch(filteredItems, (val) => {
  dragList.value = [...val];
}, { immediate: true });

// 拖拽事件
function onDragStart() {
  isDragging.value = true;
}

function onDragEnd() {
  isDragging.value = false;
  const ids = dragList.value.map(i => i.id);
  clipboardStore.reorderItems(ids);
}

// 删除项目
function deleteItem(id: string) {
  clipboardStore.removeItem(id);
}

// 更新优先级
function updatePriority(item: ClipboardItem, priority: 1 | 2 | 3) {
  item.priority = priority;
  clipboardStore.updateItem(item);
}

// 右键菜单
const contextMenuShow = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuItem = ref<ClipboardItem | null>(null);

function handleItemContextMenu(e: MouseEvent, item: ClipboardItem) {
  e.preventDefault();
  contextMenuItem.value = item;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  contextMenuShow.value = true;
}

// 编辑模式
const isEditing = ref(false);
const editTitle = ref('');
const editContent = ref('');

async function handleContextMenuSelect(key: string) {
  contextMenuShow.value = false;
  if (!contextMenuItem.value) return;
  const item = contextMenuItem.value;

  if (key === 'copy') {
    if (item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success(t('messages.imageCopied'));
    } else {
      await navigator.clipboard.writeText(item.content);
      message.success(t('messages.copied'));
    }
  } else if (key === 'edit') {
    isEditing.value = true;
    editTitle.value = item.title;
    editContent.value = item.content;
  } else if (key === 'delete') {
    deleteItem(item.id);
  }
}

function saveEdit() {
  if (!contextMenuItem.value) return;
  const item = contextMenuItem.value;
  const updated: ClipboardItem = {
    ...item,
    title: editTitle.value.trim() || item.title,
    content: editContent.value,
  };
  clipboardStore.updateItem(updated);
  isEditing.value = false;
  message.success(t('messages.saved'));
}

function cancelEdit() {
  isEditing.value = false;
  editTitle.value = '';
  editContent.value = '';
}
</script>

<template>
  <div class="clipboard-list" :class="{ 'compact-list': props.compact, 'stacked-list': props.stacked }" :style="stackStyle">
    <div v-if="filteredItems.length === 0" class="empty">
      {{ t('empty.noClipboard') }}
    </div>
    <draggable
      v-else
      v-model="dragList"
      :disabled="!dragEnabled"
      item-key="id"
      ghost-class="ghost"
      chosen-class="chosen"
      drag-class="dragging"
      :animation="200"
      :force-fallback="true"
      :fallback-tolerance="3"
      filter=".cross-app-drag-handle"
      :prevent-on-filter="false"
      class="drag-container"
      @start="onDragStart"
      @end="onDragEnd"
    >
      <template #item="{ element }">
        <div class="item-wrapper">
          <ClipboardItemCard
            :item="element"
            :compact="props.compact"
            :stacked="props.stacked"
            :show-checkbox="selectMode || selectedIds.size > 0"
            :selected="selectedIds.has(element.id)"
            :selection-anchor="selectionAnchor"
            @delete="deleteItem"
            @update-priority="updatePriority"
            @contextmenu="handleItemContextMenu($event, element)"
            @toggle-select="toggleSelect"
            @enter-select-mode="enterSelectModeFromCard"
            @move-to-category="moveToCategory"
            @batch-move-to-category="batchMoveToCategory"
            @batch-lock="batchLock"
            @batch-delete="deleteSelected"
          />
        </div>
      </template>
    </draggable>

    <!-- 右键菜单 -->
    <NDropdown
      placement="bottom-start"
      trigger="manual"
      :x="contextMenuX"
      :y="contextMenuY"
      :show="contextMenuShow"
      :options="[
        { label: t('contextMenu.copy'), key: 'copy', icon: () => h(NIcon, { component: CopyIcon, size: 14 }) },
        ...(contextMenuItem && !contextMenuItem.imageBase64 ? [
          { label: t('contextMenu.edit'), key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
        ] : []),
        { type: 'divider', key: 'd1' },
        { label: t('contextMenu.delete'), key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) }
      ]"
      @select="handleContextMenuSelect"
      @clickoutside="contextMenuShow = false"
      style="z-index: 3000"
    />

    <!-- 编辑弹窗 -->
    <NDropdown
      placement="bottom-start"
      trigger="manual"
      :x="contextMenuX"
      :y="contextMenuY"
      :show="isEditing"
      :options="[]"
      @clickoutside="cancelEdit"
      style="z-index: 3000"
    >
      <template #default>
        <div class="edit-popup" @click.stop>
          <NInput v-model:value="editTitle" size="small" placeholder="Title" class="edit-title-input" @keyup.enter="saveEdit" />
          <NInput v-model:value="editContent" type="textarea" size="small" placeholder="Content" :autosize="{ minRows: 2, maxRows: 6 }" class="edit-content-input" />
          <div class="edit-actions">
            <button class="edit-btn save" @click="saveEdit">{{ t('messages.save') }}</button>
            <button class="edit-btn cancel" @click="cancelEdit">{{ t('messages.cancel') }}</button>
          </div>
        </div>
      </template>
    </NDropdown>
  </div>
</template>

<style scoped>
.clipboard-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px 6px;
}

.clipboard-list.compact-list {
  padding: 0;
}

.clipboard-list::-webkit-scrollbar {
  width: 4px;
}

.clipboard-list::-webkit-scrollbar-track {
  background: transparent;
}

.clipboard-list::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 10px;
  transition: background 0.3s;
}

.clipboard-list:hover::-webkit-scrollbar-thumb {
  background: rgba(100, 100, 100, 0.2);
}

html.dark .clipboard-list:hover::-webkit-scrollbar-thumb {
  background: rgba(80, 80, 80, 0.4);
}

.drag-container {
  display: flex;
  flex-direction: column;
  overflow: visible;
}

.item-wrapper {
  width: 100%;
  margin-bottom: 10px;
  user-select: none;
}

.empty {
  text-align: center;
  padding: 40px;
  color: #888;
}

/* 拖拽动画 */
.ghost {
  opacity: 0.2;
  border: 2px dashed #4A90D9;
  border-radius: 12px;
  background: rgba(74, 144, 217, 0.05);
}

.chosen {
  opacity: 0.95;
  box-shadow: 0 8px 24px rgba(74, 144, 217, 0.2);
}

.dragging {
  opacity: 1;
  box-shadow: 0 16px 40px rgba(74, 144, 217, 0.3);
  border-radius: 12px;
  transform: scale(1.03) rotate(1deg);
}

/* 层叠模式间距 */
.stacked-list .item-wrapper {
  margin-bottom: calc(-80px + var(--stack-gap, 64px));
  transition: transform 0.2s ease, z-index 0s;
}

.stacked-list .item-wrapper:last-child {
  margin-bottom: 0;
}

.stacked-list .item-wrapper:nth-child(odd) {
  margin-left: 2px;
}

.stacked-list .item-wrapper:nth-child(even) {
  margin-left: -2px;
}

.stacked-list .item-wrapper:hover {
  z-index: 100;
  transform: translateY(-8px) scale(1.02);
}

/* 层叠模式下禁用卡片自身 hover transform（穿透子组件 scoped） */
:deep(.task-card.stacked:hover) {
  transform: none !important;
}

/* 编辑弹窗 */
.edit-popup {
  padding: 12px;
  background: #fff;
  border: 1px solid #e0e0e0;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  min-width: 280px;
  max-width: 400px;
}

html.dark .edit-popup {
  background: #2a2a2a;
  border-color: #444;
}

.edit-popup .edit-title-input {
  margin-bottom: 8px;
}

.edit-popup .edit-content-input {
  margin-bottom: 8px;
}

.edit-popup .edit-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.edit-popup .edit-btn {
  padding: 4px 16px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.edit-popup .edit-btn.save {
  background: #4A90D9;
  color: #fff;
}

.edit-popup .edit-btn.save:hover {
  background: #3A7BC8;
}

.edit-popup .edit-btn.cancel {
  background: transparent;
  color: #999;
  border: 1px solid #ddd;
}

.edit-popup .edit-btn.cancel:hover {
  background: rgba(0, 0, 0, 0.05);
}

html.dark .edit-popup .edit-btn.cancel {
  border-color: #444;
  color: #888;
}

html.dark .edit-popup .edit-btn.cancel:hover {
  background: rgba(255, 255, 255, 0.05);
}
</style>
