<script setup lang="ts">
import { ref, computed, watch, h } from 'vue';
import { NDropdown, NIcon, NInput, NPopconfirm, useMessage } from 'naive-ui';
import { TrashOutline as DeleteIcon, CopyOutline as CopyIcon, StarOutline as StarIcon, Star as StarFilledIcon, CreateOutline as EditIcon, TimeOutline as TimeIcon, CheckmarkCircleOutline as CheckAllIcon, CloseOutline as CloseIcon } from '@vicons/ionicons5';
import draggable from 'vuedraggable';
import { useClipboardStore } from '../stores/clipboardStore';
import ClipboardItemCard from './ClipboardItemCard.vue';
import type { ClipboardItem } from '../types';
import { BUILTIN_CLIPBOARD_CATEGORIES } from '../types';

const clipboardStore = useClipboardStore();
const message = useMessage();

const props = defineProps<{
  compact?: boolean;
  categoryFilter?: string | null;
  stacked?: boolean;
}>();

const isDragging = ref(false);
const dragList = ref<ClipboardItem[]>([]);

// 批量选择
const selectMode = ref(false);
const selectedIds = ref(new Set<string>());
const selectionAnchor = ref<string | null>(null); // Shift 连选的锚点

// 切换选择模式
function toggleSelectMode() {
  selectMode.value = !selectMode.value;
  if (!selectMode.value) {
    selectedIds.value = new Set();
    selectionAnchor.value = null;
  }
  listMenuShow.value = false;
}

// 切换单个选中
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

// 移动分类
function moveToCategory(item: ClipboardItem, categoryId: string) {
  item.categoryId = categoryId;
  clipboardStore.updateItem(item);
}

// 全选 / 取消全选
function selectAll() {
  selectedIds.value = new Set(filteredItems.value.map(i => i.id));
  if (!selectMode.value) selectMode.value = true;
}

// 批量删除
async function deleteSelected() {
  const ids = [...selectedIds.value];
  if (ids.length === 0) return;
  await clipboardStore.removeItems(ids);
  message.success(`已删除 ${ids.length} 项`);
  selectedIds.value = new Set();
  if (ids.length === filteredItems.value.length) {
    selectMode.value = false;
  }
}

// 过滤后的项目（精简模式下使用 props.categoryFilter，否则用 store 的过滤）
const filteredItems = computed(() => {
  let items = [...clipboardStore.items];

  if (props.compact && props.categoryFilter) {
    // 精简模式：使用传入的分类过滤
    items = items.filter(i => i.categoryId === props.categoryFilter);
  } else if (props.compact) {
    // 精简模式无过滤：显示全部
    // no filter, show all
  } else {
    // 正常模式：使用 store 的 filteredItems（包含分类和搜索）
    return clipboardStore.filteredItems;
  }

  if (clipboardStore.searchQuery) {
    const q = clipboardStore.searchQuery.toLowerCase();
    items = items.filter(i =>
      i.title.toLowerCase().includes(q) ||
      i.content.toLowerCase().includes(q)
    );
  }

  items.sort((a, b) => a.sortOrder - b.sortOrder);
  return items;
});

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

// 清理已过期项目
async function cleanupExpired() {
  const count = await clipboardStore.cleanupExpiredItems();
  if (count > 0) {
    message.success(`已清理 ${count} 个过期项目`);
  } else {
    message.info('没有已过期项目');
  }
}

// 列表右键菜单
const listMenuShow = ref(false);
const listMenuX = ref(0);
const listMenuY = ref(0);

function handleListContextMenu(e: MouseEvent) {
  e.preventDefault();
  listMenuX.value = e.clientX;
  listMenuY.value = e.clientY;
  listMenuShow.value = true;
}

async function handleListMenuSelect(key: string) {
  listMenuShow.value = false;
  if (key === 'cleanup') {
    await cleanupExpired();
  }
}

async function handleContextMenuSelect(key: string) {
  contextMenuShow.value = false;
  if (!contextMenuItem.value) return;
  const item = contextMenuItem.value;

  if (key === 'copy') {
    if (item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success('已复制图片');
    } else {
      await navigator.clipboard.writeText(item.content);
      message.success('已复制');
    }
  } else if (key === 'favorite') {
    const result = await clipboardStore.favoriteItem(item);
    if (result === 'favorited') {
      message.success('已收藏');
    } else if (result === 'unfavorited') {
      message.success('已取消收藏');
    } else {
      message.error('收藏分类不存在');
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
  message.success('已保存');
}

function cancelEdit() {
  isEditing.value = false;
  editTitle.value = '';
  editContent.value = '';
}
</script>

<template>
  <div class="clipboard-list" :class="{ 'compact-list': props.compact, 'stacked-list': props.stacked }" @contextmenu="handleListContextMenu">
    <!-- 选择工具栏 -->
    <div v-if="selectMode || selectedIds.size > 0" class="selection-toolbar">
      <span class="selection-count">已选 {{ selectedIds.size }} / {{ filteredItems.length }}</span>
      <button class="toolbar-btn" @click="selectAll" :disabled="selectedIds.size === filteredItems.length" title="全选">
        <NIcon :component="CheckAllIcon" size="16" />
        全选
      </button>
      <NPopconfirm @positive-click="deleteSelected">
        <template #trigger>
          <button class="toolbar-btn danger" :disabled="selectedIds.size === 0" title="删除选中">
            <NIcon :component="DeleteIcon" size="16" />
            删除选中
          </button>
        </template>
        确定删除选中的 {{ selectedIds.size }} 项吗？
      </NPopconfirm>
      <button class="toolbar-btn" @click="toggleSelectMode" title="退出选择模式">
        <NIcon :component="CloseIcon" size="14" />
        退出
      </button>
    </div>

    <div v-if="filteredItems.length === 0" class="empty">
      暂无剪贴板记录
      <button class="cleanup-btn" @click="cleanupExpired">清理已过期项目</button>
    </div>
    <draggable
      v-else
      v-model="dragList"
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
        { label: '复制内容', key: 'copy', icon: () => h(NIcon, { component: CopyIcon, size: 14 }) },
        { label: contextMenuItem?.categoryId === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE ? '取消收藏' : '收藏', key: 'favorite', icon: () => h(NIcon, { component: contextMenuItem?.categoryId === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE ? StarFilledIcon : StarIcon, size: 14, style: { color: contextMenuItem?.categoryId === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE ? '#F39C12' : '#333' } }) },
        ...(contextMenuItem && !contextMenuItem.imageBase64 ? [
          { label: '编辑', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
        ] : []),
        { type: 'divider', key: 'd1' },
        { label: '删除', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) }
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
          <NInput v-model:value="editTitle" size="small" placeholder="标题" class="edit-title-input" @keyup.enter="saveEdit" />
          <NInput v-model:value="editContent" type="textarea" size="small" placeholder="内容" :autosize="{ minRows: 2, maxRows: 6 }" class="edit-content-input" />
          <div class="edit-actions">
            <button class="edit-btn save" @click="saveEdit">保存</button>
            <button class="edit-btn cancel" @click="cancelEdit">取消</button>
          </div>
        </div>
      </template>
    </NDropdown>

    <!-- 列表右键菜单 -->
    <NDropdown
      placement="bottom-start"
      trigger="manual"
      :x="listMenuX"
      :y="listMenuY"
      :show="listMenuShow"
      :options="[
        { label: '清理已过期项目', key: 'cleanup', icon: () => h(NIcon, { component: TimeIcon, size: 14 }) }
      ]"
      @select="handleListMenuSelect"
      @clickoutside="listMenuShow = false"
      style="z-index: 3100"
    />
  </div>
</template>

<style scoped>
.selection-toolbar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin-bottom: 8px;
  background: #f0f5ff;
  border: 1px solid #d0e0f5;
  border-radius: 10px;
  flex-shrink: 0;
}

html.dark .selection-toolbar {
  background: #1e2a3a;
  border-color: #2a3a4a;
}

.selection-count {
  font-size: 13px;
  font-weight: 600;
  color: #4A90D9;
  margin-right: auto;
}

.toolbar-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  border: 1px solid #d0d0d0;
  border-radius: 6px;
  background: #fff;
  color: #333;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.toolbar-btn:hover:not(:disabled) {
  background: #4A90D9;
  border-color: #4A90D9;
  color: #fff;
}

.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.toolbar-btn.danger {
  color: #E05252;
  border-color: #E05252;
}

.toolbar-btn.danger:hover:not(:disabled) {
  background: #E05252;
  color: #fff;
}

html.dark .toolbar-btn {
  background: #2a2a2a;
  border-color: #444;
  color: #ccc;
}

html.dark .toolbar-btn:hover:not(:disabled) {
  background: #4A90D9;
  border-color: #4A90D9;
  color: #fff;
}

html.dark .toolbar-btn.danger {
  color: #E05252;
  border-color: #E05252;
}

html.dark .toolbar-btn.danger:hover:not(:disabled) {
  background: #E05252;
  color: #fff;
}

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

.cleanup-btn {
  display: block;
  margin: 12px auto 0;
  padding: 4px 16px;
  background: transparent;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 12px;
  color: #999;
  cursor: pointer;
  transition: all 0.15s;
}

.cleanup-btn:hover {
  border-color: #E05252;
  color: #E05252;
}

html.dark .cleanup-btn {
  border-color: #444;
  color: #777;
}

html.dark .cleanup-btn:hover {
  border-color: #E05252;
  color: #E05252;
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
  margin-bottom: -10px;
  transition: transform 0.2s ease, z-index 0s;
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
