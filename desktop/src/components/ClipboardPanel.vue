<script setup lang="ts">
import { ref, computed, watch, h } from 'vue';
import { NDropdown, NIcon, NInput, useMessage } from 'naive-ui';
import { TrashOutline as DeleteIcon, CopyOutline as CopyIcon, StarOutline as StarIcon, Star as StarFilledIcon, CreateOutline as EditIcon } from '@vicons/ionicons5';
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
}>();

const isDragging = ref(false);
const dragList = ref<ClipboardItem[]>([]);

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
  <div class="clipboard-list" :class="{ 'compact-list': props.compact }">
    <div v-if="filteredItems.length === 0" class="empty">暂无剪贴板记录</div>
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
      class="drag-container"
      @start="onDragStart"
      @end="onDragEnd"
    >
      <template #item="{ element }">
        <div class="item-wrapper">
          <ClipboardItemCard
            :item="element"
            :compact="props.compact"
            @delete="deleteItem"
            @update-priority="updatePriority"
            @contextmenu="handleItemContextMenu($event, element)"
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
