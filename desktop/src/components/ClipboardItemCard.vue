<script setup lang="ts">
import { NIcon, NDropdown } from 'naive-ui';
import { h } from 'vue';
import {
  TrashOutline as DeleteIcon, CopyOutline as CopyIcon
} from '@vicons/ionicons5';
import type { ClipboardItem } from '../types';
import { useMessage } from 'naive-ui';
import { ref } from 'vue';

const props = defineProps<{
  item: ClipboardItem;
}>();

const emit = defineEmits<{
  (e: 'delete', id: string): void;
  (e: 'update-priority', item: ClipboardItem, priority: 1 | 2 | 3): void;
  (e: 'contextmenu', event: MouseEvent, item: ClipboardItem): void;
}>();

const message = useMessage();
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

const contextMenuOptions = [
  { label: '复制', key: 'copy', icon: () => h(NIcon, { component: CopyIcon, size: 16 }) },
  { type: 'divider', key: 'd1' },
  { label: '删除', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } }) },
];

async function copyContent() {
  try {
    if (props.item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success('已复制图片');
    } else if (props.item.content) {
      await navigator.clipboard.writeText(props.item.content);
      message.success('已复制');
    }
  } catch (e) {
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

function handleClick() {
  copyContent();
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  showContextMenu.value = true;
}

function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'copy') copyContent();
  if (key === 'delete') emit('delete', props.item.id);
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
  <div class="task-card" @click="handleClick" @contextmenu="handleContextMenu">
    <div class="task-content">
      <!-- 纯图片：只显示图片 -->
      <div v-if="item.imageBase64" class="image-only">
        <div class="task-thumbnail">
          <img :src="item.imageBase64" alt="剪贴板图片" />
        </div>
      </div>

      <!-- 文本：标题 + 内容 -->
      <div v-if="!item.imageBase64">
        <div class="task-title">{{ item.title }}</div>
        <div v-if="item.content" class="task-desc">{{ item.content }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-card {
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  border-left: 3px solid #4A90D9;
  font-size: var(--task-font-size, 14px);
  font-family: var(--task-font-family, inherit);
  transition: box-shadow 0.2s;
  cursor: pointer;
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
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.task-card:active {
  transform: scale(0.99);
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
}

html.dark .task-desc {
  color: #999;
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
</style>
