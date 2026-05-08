<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';
import draggable from 'vuedraggable';
import { useCategoryStore } from '../stores/categoryStore';
import type { Category } from '../types';
import { NDropdown, NPopover, NColorPicker, useMessage } from 'naive-ui';
import { h } from 'vue';
import { NIcon } from 'naive-ui';
import { CreateOutline as EditIcon, TrashOutline as DeleteIcon } from '@vicons/ionicons5';
import { FREE_CATEGORY_LIMIT } from '../types';

const store = useCategoryStore();
const message = useMessage();

// 独立的 ref，由 vuedraggable 控制
const categoryList = ref<Category[]>([]);
const tabsRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);

watch(() => store.categories, (val) => {
  categoryList.value = [...val];
}, { immediate: true });

function onDragStart() {
  isDragging.value = true;
}

function onDragEnd() {
  isDragging.value = false;
  const ids = categoryList.value.map(c => c.id);
  store.reorder(ids);
}

function selectCategory(id: string | null) {
  if (isDragging.value) return;
  store.select(id);
}

// 内联编辑状态
const editingCategoryId = ref('');
const editingName = ref('');

function startInlineEdit(cat: Category) {
  editingCategoryId.value = cat.id;
  editingName.value = cat.name;
}

async function finishInlineEdit(cat: Category) {
  const newName = editingName.value.trim();
  if (!newName) {
    message.warning('分类名称不能为空');
    return;
  }
  cat.name = newName;
  await store.update(cat);
  editingCategoryId.value = '';
}

// 右键菜单
const contextMenuShow = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuCat = ref<Category | null>(null);
const colorPickerShow = ref(false);

const presetColors = [
  '#4A90D9', '#E05252', '#FF9500', '#28C840',
  '#9B59B6', '#3498DB', '#1ABC9C', '#F39C12'
];

function handleTabContextMenu(e: MouseEvent, cat: Category) {
  e.preventDefault();
  contextMenuCat.value = cat;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  contextMenuShow.value = true;
}

function handleContextMenuSelect(key: string) {
  contextMenuShow.value = false;
  if (!contextMenuCat.value) return;
  const cat = contextMenuCat.value;

  if (key === 'edit') {
    startInlineEdit(cat);
  } else if (key === 'color') {
    colorPickerShow.value = true;
  } else if (key === 'delete') {
    deleteCategory(cat);
  }
}

function handleColorChange(color: string) {
  if (contextMenuCat.value) {
    contextMenuCat.value.color = color;
    store.update(contextMenuCat.value);
  }
}

// 删除分类
import { useDialog } from 'naive-ui';
const dialog = useDialog();

function deleteCategory(cat: Category) {
  dialog.warning({
    title: '确认删除',
    content: `确定删除分类"${cat.name}"及其所有任务？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await store.remove(cat.id);
      message.success('删除成功');
    }
  });
}

// 快速新建分类
async function quickAddCategory() {
  if (!store.canAddMore) {
    message.warning(`分类数量已达上限（${FREE_CATEGORY_LIMIT}个）`);
    return;
  }
  const cat = await store.add('New', '#4A90D9');
  // 新建后进入编辑状态
  editingCategoryId.value = cat.id;
  editingName.value = 'New';
  store.select(cat.id);
}

// 鼠标滚轮横向滚动
function handleWheel(e: WheelEvent) {
  if (tabsRef.value) {
    e.preventDefault();
    tabsRef.value.scrollLeft += e.deltaY * 2;
  }
}

onMounted(() => {
  if (tabsRef.value) {
    tabsRef.value.addEventListener('wheel', handleWheel, { passive: false });
  }
});

onUnmounted(() => {
  if (tabsRef.value) {
    tabsRef.value.removeEventListener('wheel', handleWheel);
  }
});
</script>

<template>
  <div class="category-tabs" ref="tabsRef">
    <!-- "全部" tab，固定不可拖拽 -->
    <button
      :class="['tab-btn', 'all-tab', { active: !store.selectedCategoryId }]"
      @click="selectCategory(null)"
    >
      全部
    </button>

    <!-- 可拖拽的分类 tab -->
    <draggable
      v-if="categoryList.length > 1"
      v-model="categoryList"
      item-key="id"
      :animation="200"
      :force-fallback="true"
      :fallback-tolerance="3"
      ghost-class="ghost-tab"
      chosen-class="chosen-tab"
      drag-class="dragging-tab"
      class="draggable-tabs"
      :class="{ 'is-dragging': isDragging }"
      @start="onDragStart"
      @end="onDragEnd"
    >
      <template #item="{ element }">
        <button
          :class="['tab-btn', { active: store.selectedCategoryId === element.id }]"
          @click="selectCategory(element.id)"
          @contextmenu="handleTabContextMenu($event, element)"
        >
          <template v-if="editingCategoryId === element.id">
            <input
              ref="editInputRef"
              v-model="editingName"
              class="tab-edit-input"
              @blur="finishInlineEdit(element)"
              @keyup.enter="finishInlineEdit(element)"
              @keyup.escape="editingCategoryId = ''"
              @click.stop
            />
          </template>
          <template v-else>
            <span :style="element.color ? { color: element.color } : {}">{{ element.name }}</span>
          </template>
        </button>
      </template>
    </draggable>

    <!-- 只有一个分类时不需要拖拽 -->
    <template v-else>
      <button
        v-for="cat in categoryList"
        :key="cat.id"
        :class="['tab-btn', { active: store.selectedCategoryId === cat.id }]"
        @click="selectCategory(cat.id)"
        @contextmenu="handleTabContextMenu($event, cat)"
      >
        <template v-if="editingCategoryId === cat.id">
          <input
            v-model="editingName"
            class="tab-edit-input"
            @blur="finishInlineEdit(cat)"
            @keyup.enter="finishInlineEdit(cat)"
            @keyup.escape="editingCategoryId = ''"
            @click.stop
          />
        </template>
        <template v-else>
          <span :style="cat.color ? { color: cat.color } : {}">{{ cat.name }}</span>
        </template>
      </button>
    </template>

    <!-- 添加分类按钮 -->
    <button
      class="tab-btn add-tab-btn"
      @click="quickAddCategory"
      :disabled="!store.canAddMore"
    >
      ➕
    </button>

    <!-- 右键菜单 -->
    <NDropdown
      placement="bottom-start"
      trigger="manual"
      :x="contextMenuX"
      :y="contextMenuY"
      :show="contextMenuShow"
      :options="[
        { label: '编辑名称', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
        { label: '设置颜色', key: 'color' },
        { type: 'divider', key: 'd1' },
        { label: '删除分类', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) }
      ]"
      @select="handleContextMenuSelect"
      @clickoutside="contextMenuShow = false"
    />

    <!-- 颜色选择弹窗 -->
    <NPopover
      trigger="manual"
      :show="colorPickerShow"
      :x="contextMenuX"
      :y="contextMenuY + 30"
      @clickoutside="colorPickerShow = false"
    >
      <NColorPicker
        v-model:value="contextMenuCat!.color"
        :swatches="presetColors"
        :modes="['hex']"
        @update:value="handleColorChange"
        style="width: 220px"
      />
    </NPopover>
  </div>
</template>

<style scoped>
.category-tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  overflow-y: hidden;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.category-tabs::-webkit-scrollbar {
  display: none;
}

.draggable-tabs {
  display: flex;
  gap: 4px;
}

.tab-btn {
  padding: 4px 12px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #ccc;
  cursor: pointer;
  font-size: 14px;
  white-space: nowrap;
  transition: background 0.15s, color 0.15s;
  user-select: none;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  display: flex;
  align-items: center;
}

.tab-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

.tab-btn.active {
  color: #4A90D9;
  border-bottom: 2px solid #4A90D9;
  border-radius: 4px 4px 0 0;
}

.all-tab.active {
  background: rgba(74, 144, 217, 0.2);
  font-weight: 600;
}

.add-tab-btn {
  padding: 4px 8px;
  font-size: 12px;
  opacity: 0.6;
}

.add-tab-btn:hover {
  opacity: 1;
  background: rgba(255, 255, 255, 0.08);
}

.add-tab-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.tab-edit-input {
  background: transparent;
  border: 1px solid #4A90D9;
  border-radius: 4px;
  padding: 0 4px;
  font-size: 14px;
  color: #fff;
  outline: none;
  width: 80px;
  font-family: inherit;
}

html.dark .tab-edit-input {
  color: #e0e0e0;
  background: #2a2a2a;
}

/* 拖拽样式 */
.ghost-tab {
  opacity: 0.3;
  background: rgba(74, 144, 217, 0.1);
  border-radius: 4px;
}

.chosen-tab {
  opacity: 0.9;
  transform: scale(1.05);
}

.dragging-tab {
  opacity: 1;
  box-shadow: 0 4px 12px rgba(74, 144, 217, 0.3);
  background: rgba(74, 144, 217, 0.15);
  border-radius: 4px;
}

.draggable-tabs.is-dragging .tab-btn:hover {
  background: transparent;
}
</style>
