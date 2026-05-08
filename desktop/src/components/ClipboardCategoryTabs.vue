<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';
import draggable from 'vuedraggable';
import { useClipboardStore } from '../stores/clipboardStore';
import type { ClipboardCategory } from '../types';
import { NDropdown, NPopover, NColorPicker, useMessage, useDialog, type DropdownOption } from 'naive-ui';
import { h } from 'vue';
import { NIcon } from 'naive-ui';
import { CreateOutline as EditIcon, TrashOutline as DeleteIcon, StarOutline as StarIcon } from '@vicons/ionicons5';
import { FREE_CATEGORY_LIMIT, isBuiltinClipboardCategory, BUILTIN_CLIPBOARD_CATEGORIES } from '../types';

const store = useClipboardStore();
const message = useMessage();
const dialog = useDialog();

// 内置分类始终存在，由 store.builtinCategories 提供
const builtinList = ref<ClipboardCategory[]>([]);
// 用户自定义分类，可拖拽
const customList = ref<ClipboardCategory[]>([]);
const tabsRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);

watch(() => store.builtinCategories, (val) => {
  builtinList.value = [...val];
}, { immediate: true });

watch(() => store.customCategories, (val) => {
  customList.value = [...val];
}, { immediate: true });

function onDragStart() {
  isDragging.value = true;
}

function onDragEnd() {
  isDragging.value = false;
  const ids = customList.value.map(c => c.id);
  store.reorderCategories(ids);
}

function selectCategory(id: string | null) {
  if (isDragging.value) return;
  store.selectCategory(id);
}

// 内联编辑状态
const editingCategoryId = ref('');
const editingName = ref('');

function startInlineEdit(cat: ClipboardCategory) {
  editingCategoryId.value = cat.id;
  editingName.value = cat.name;
}

async function finishInlineEdit(cat: ClipboardCategory) {
  const newName = editingName.value.trim();
  if (!newName) {
    message.warning('分类名称不能为空');
    return;
  }
  cat.name = newName;
  await store.updateCategory(cat);
  editingCategoryId.value = '';
}

// 右键菜单
const contextMenuShow = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuCat = ref<ClipboardCategory | null>(null);
const colorPickerShow = ref(false);

const presetColors = [
  '#4A90D9', '#E05252', '#FF9500', '#28C840',
  '#9B59B6', '#3498DB', '#1ABC9C', '#F39C12'
];

function handleTabContextMenu(e: MouseEvent, cat: ClipboardCategory) {
  e.preventDefault();
  contextMenuCat.value = cat;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  contextMenuShow.value = true;
}

// 右键菜单选项：内置分类不显示删除
const contextMenuOptions = (cat: ClipboardCategory): DropdownOption[] => {
  const isBuiltin = isBuiltinClipboardCategory(cat.id);
  if (isBuiltin) {
    return [
      { label: '编辑名称', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
      { label: '设置颜色', key: 'color' },
    ];
  }
  return [
    { label: '编辑名称', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 14 }) },
    { label: '设置颜色', key: 'color' },
    { type: 'divider', key: 'd1' },
    { label: '删除分类', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 14, style: { color: '#E05252' } }) },
  ];
};

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
    store.updateCategory(contextMenuCat.value);
  }
}

function deleteCategory(cat: ClipboardCategory) {
  dialog.warning({
    title: '确认删除',
    content: `确定删除分类"${cat.name}"及其所有剪贴板项目？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await store.removeCategory(cat.id);
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
  const cat = await store.addCategory('New', '#4A90D9');
  editingCategoryId.value = cat.id;
  editingName.value = 'New';
  store.selectCategory(cat.id);
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
    <!-- 全部 tab -->
    <button
      :class="['tab-btn', 'all-tab', { active: store.selectedCategoryId === null }]"
      @click="selectCategory(null)"
    >
      全部
    </button>

    <!-- 内置分类 tab：文本、图像、收藏，固定不可拖拽 -->
    <button
      v-for="cat in builtinList"
      :key="cat.id"
      :class="['tab-btn', 'builtin-tab', { active: store.selectedCategoryId === cat.id }]"
      @click="selectCategory(cat.id)"
      @contextmenu="handleTabContextMenu($event, cat)"
    >
      <!-- 收藏分类显示特殊图标 -->
      <NIcon v-if="cat.id === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE" :component="StarIcon" size="14" :style="{ marginRight: '4px', color: store.selectedCategoryId === cat.id ? cat.color : cat.color }" />
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

    <!-- 可拖拽的用户自定义分类 tab -->
    <draggable
      v-if="customList.length > 1"
      v-model="customList"
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

    <!-- 只有一个自定义分类时不需要拖拽 -->
    <template v-else>
      <button
        v-for="cat in customList"
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
      :options="contextMenuCat ? contextMenuOptions(contextMenuCat) : []"
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

/* 内置分类样式：更突出的背景 */
.builtin-tab.active {
  background: rgba(74, 144, 217, 0.15);
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
