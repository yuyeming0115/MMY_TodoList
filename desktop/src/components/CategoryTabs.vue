<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';
import draggable from 'vuedraggable';
import { useCategoryStore } from '../stores/categoryStore';
import type { Category } from '../types';

const store = useCategoryStore();

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
  store.select(id);
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
      :class="['tab-btn', { active: !store.selectedCategoryId }]"
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
        >
          <span :style="element.color ? { color: element.color } : {}">{{ element.name }}</span>
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
      >
        <span :style="cat.color ? { color: cat.color } : {}">{{ cat.name }}</span>
      </button>
    </template>
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
  /* 隐藏滚动条但保持可滚动 */
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
}

.tab-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

.tab-btn.active {
  color: #4A90D9;
  border-bottom: 2px solid #4A90D9;
  border-radius: 4px 4px 0 0;
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

/* 拖拽中禁用 hover 效果 */
.draggable-tabs.is-dragging .tab-btn:hover {
  background: transparent;
}
</style>