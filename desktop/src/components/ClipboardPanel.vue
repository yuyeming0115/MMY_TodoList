<script setup lang="ts">
import { ref, computed, watch, h } from 'vue';
import { NDropdown, NIcon, useMessage } from 'naive-ui';
import { TrashOutline as DeleteIcon, CopyOutline as CopyIcon } from '@vicons/ionicons5';
import draggable from 'vuedraggable';
import { useClipboardStore } from '../stores/clipboardStore';
import ClipboardItemCard from './ClipboardItemCard.vue';
import type { ClipboardItem } from '../types';

const clipboardStore = useClipboardStore();
const message = useMessage();

const isDragging = ref(false);
const dragList = ref<ClipboardItem[]>([]);

// 过滤后的项目（从 store 读取）
const filteredItems = computed(() => clipboardStore.filteredItems);

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

async function handleContextMenuSelect(key: string) {
  contextMenuShow.value = false;
  if (!contextMenuItem.value) return;
  const item = contextMenuItem.value;

  if (key === 'copy') {
    await navigator.clipboard.writeText(item.content);
    message.success('已复制');
  } else if (key === 'delete') {
    deleteItem(item.id);
  }
}
</script>

<template>
  <div class="clipboard-list">
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
        { type: 'divider', key: 'd1' },
        { label: '删除', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) }
      ]"
      @select="handleContextMenuSelect"
      @clickoutside="contextMenuShow = false"
    />
  </div>
</template>

<style scoped>
.clipboard-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px 0;
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
</style>
