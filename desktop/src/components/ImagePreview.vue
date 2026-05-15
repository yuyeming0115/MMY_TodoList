<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue';
import { NIcon, NSlider } from 'naive-ui';
import { ExpandOutline as ExpandIcon, ResizeOutline as ResizeIcon } from '@vicons/ionicons5';

const props = defineProps<{
  src: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: 'show-request'): void;
  (e: 'hide-request'): void;
}>();

const containerRef = ref<HTMLElement | null>(null);
const imageRef = ref<HTMLImageElement | null>(null);

// 缩放级别：'fit' | 0.25 | 0.5 | 1.0
const zoomMode = ref<'fit' | number>('fit');

// 滑杆值（百分比，对应 25%-200%）
const sliderValue = ref(100);

// 当前应用的缩放比例

// 图片样式
const imageStyle = computed(() => {
  if (zoomMode.value === 'fit') {
    return {
      width: '100%',
      height: 'auto',
      maxWidth: '600px',
      maxHeight: '500px',
      objectFit: 'contain' as const,
    };
  }
  const scale = zoomMode.value;
  return {
    transform: `scale(${scale})`,
    transformOrigin: 'top left',
  };
});

// 工具栏容器样式（缩放模式下需要足够宽）
const toolbarStyle = computed(() => {
  if (zoomMode.value === 'fit') return {};
  const w = Math.max(300, 600 * zoomMode.value);
  return { minWidth: `${Math.min(w, 800)}px` };
});

function setZoom(mode: 'fit' | number) {
  zoomMode.value = mode;
  if (mode !== 'fit' && typeof mode === 'number') {
    sliderValue.value = Math.round(mode * 100);
  }
}

function onSliderChange(val: number) {
  const scale = val / 100;
  zoomMode.value = scale;
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? -0.05 : 0.05;
  const current = zoomMode.value === 'fit' ? 1 : zoomMode.value;
  const next = Math.max(0.1, Math.min(3, current + delta));
  zoomMode.value = next;
  sliderValue.value = Math.round(next * 100);
}

function cancelHidePreview() {
  emit('show-request');
}

function startHideTimer() {
  emit('hide-request');
}

function openFullScreen() {
  if (!props.src) return;
  const win = window.open('', '_blank');
  if (win) {
    win.document.write(`
      <html><head><title>图片预览</title>
      <style>body{margin:0;display:flex;align-items:center;justify-content:center;min-height:100vh;background:#000}img{max-width:100%;max-height:100vh;object-fit:contain}</style>
      </head><body><img src="${props.src}" /></body></html>
    `);
    win.document.close();
  }
  emit('hide-request');
}

// 定位浮层
function positionPopup(trigger: HTMLElement | null) {
  if (!trigger) return;
  const triggerRect = trigger.getBoundingClientRect();
  const popup = containerRef.value;
  if (!popup) return;

  nextTick(() => {
    const popupRect = popup.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    // 默认显示在缩略图右侧
    let left = triggerRect.right + 8;
    let top = triggerRect.top;

    // 右侧空间不足，显示在左侧
    if (left + popupRect.width > vw) {
      left = triggerRect.left - popupRect.width - 8;
    }

    // 确保不超出左侧
    if (left < 8) left = 8;

    // 下方空间不足，显示在上方
    if (top + popupRect.height > vh) {
      top = vh - popupRect.height - 8;
    }

    // 确保不超出顶部
    if (top < 8) top = 8;

    popup.style.left = `${left}px`;
    popup.style.top = `${top}px`;
  });
}

watch(() => props.visible, (val) => {
  if (val) {
    zoomMode.value = 'fit';
    sliderValue.value = 100;
  }
});

defineExpose({ positionPopup });
</script>

<template>
  <Teleport to="body">
    <Transition name="preview-fade">
      <div
        v-if="visible"
        ref="containerRef"
        class="image-preview-popup"
        :style="toolbarStyle"
        @mouseenter="cancelHidePreview"
        @mouseleave="startHideTimer"
        @wheel="onWheel"
      >
        <!-- 图片区域 -->
        <div class="preview-image-wrapper" :style="{ overflow: zoomMode === 'fit' ? 'visible' : 'auto' }">
          <img
            ref="imageRef"
            :src="src"
            class="preview-image"
            :style="imageStyle"
            @dblclick="openFullScreen"
          />
        </div>

        <!-- 工具栏 -->
        <div class="preview-toolbar">
          <button
            class="zoom-btn"
            :class="{ active: zoomMode === 'fit' }"
            @click="setZoom('fit')"
            :title="'适应'"
          >
            <NIcon :component="ResizeIcon" size="14" />
          </button>
          <button
            class="zoom-btn"
            :class="{ active: zoomMode === 0.25 }"
            @click="setZoom(0.25)"
          >25%</button>
          <button
            class="zoom-btn"
            :class="{ active: zoomMode === 0.5 }"
            @click="setZoom(0.5)"
          >50%</button>
          <button
            class="zoom-btn"
            :class="{ active: zoomMode === 1 }"
            @click="setZoom(1)"
          >100%</button>
          <div class="zoom-slider">
            <NSlider
              v-model:value="sliderValue"
              :min="25"
              :max="200"
              :step="5"
              @update:value="onSliderChange"
            />
          </div>
          <button
            class="zoom-btn fullscreen-btn"
            @click="openFullScreen"
            :title="'全屏查看'"
          >
            <NIcon :component="ExpandIcon" size="14" />
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.preview-fade-enter-active,
.preview-fade-leave-active {
  transition: opacity 0.15s, transform 0.15s;
}

.preview-fade-enter-from,
.preview-fade-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.image-preview-popup {
  position: fixed;
  z-index: 9999;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  border: 1px solid #e0e0e0;
  padding: 12px;
  max-width: 650px;
  transition: max-width 0.2s;
}

html.dark .image-preview-popup {
  background: #2a2a2a;
  border-color: #444;
}

.preview-image-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  max-height: 500px;
  border-radius: 8px;
  margin-bottom: 8px;
}

.preview-image {
  display: block;
  border-radius: 4px;
  cursor: zoom-in;
  user-select: none;
  -webkit-user-drag: none;
}

.preview-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-top: 8px;
  border-top: 1px solid #eee;
  flex-wrap: wrap;
}

html.dark .preview-toolbar {
  border-color: #3a3a3a;
}

.zoom-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px 10px;
  border: 1px solid #ddd;
  border-radius: 6px;
  background: #fff;
  color: #555;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  min-width: 40px;
}

.zoom-btn:hover {
  background: #4A90D9;
  border-color: #4A90D9;
  color: #fff;
}

.zoom-btn.active {
  background: #4A90D9;
  border-color: #4A90D9;
  color: #fff;
}

html.dark .zoom-btn {
  background: #333;
  border-color: #555;
  color: #ccc;
}

html.dark .zoom-btn:hover,
html.dark .zoom-btn.active {
  background: #4A90D9;
  border-color: #4A90D9;
  color: #fff;
}

.zoom-slider {
  flex: 1;
  min-width: 80px;
  max-width: 160px;
}

.fullscreen-btn {
  min-width: 36px;
}
</style>
