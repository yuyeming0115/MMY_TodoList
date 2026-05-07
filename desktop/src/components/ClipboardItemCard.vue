<script setup lang="ts">
import { NIcon, NButton, NSpace } from 'naive-ui';
import {
  Star as StarIcon, TrashOutline as DeleteIcon, CopyOutline as CopyIcon
} from '@vicons/ionicons5';
import type { ClipboardItem } from '../types';
import { useMessage } from 'naive-ui';

const props = defineProps<{
  item: ClipboardItem;
}>();

const emit = defineEmits<{
  (e: 'delete', id: string): void;
  (e: 'update-priority', item: ClipboardItem, priority: 1 | 2 | 3): void;
  (e: 'contextmenu', event: MouseEvent, item: ClipboardItem): void;
}>();

const message = useMessage();

function handleClickStar(priority: 1 | 2 | 3) {
  emit('update-priority', props.item, priority);
}

async function copyContent() {
  await navigator.clipboard.writeText(props.item.content);
  message.success('已复制');
}

function formatDate(ts: number): string {
  const d = new Date(ts);
  const month = d.getMonth() + 1;
  const day = d.getDate();
  return `${month}/${day}`;
}
</script>

<template>
  <div class="task-card" @contextmenu="$emit('contextmenu', $event, item)">
    <!-- 左侧勾选框 -->
    <div class="task-check" />

    <!-- 内容区 -->
    <div class="task-content">
      <div class="task-title">{{ item.title }}</div>
      <div v-if="item.content" class="task-desc">{{ item.content }}</div>
      <div v-if="item.imageBase64" class="task-thumbnail">
        <img :src="item.imageBase64" alt="剪贴板图片" />
      </div>
      <div class="task-meta">
        <span class="task-stars">
          <NIcon
            v-for="i in 3"
            :key="i"
            :component="StarIcon"
            size="14"
            :class="['star', { filled: i <= item.priority }]"
            @click="handleClickStar(i as 1 | 2 | 3)"
          />
        </span>
        <span class="task-time">{{ formatDate(item.createdAt) }}</span>
        <NSpace :size="4">
          <NButton text size="tiny" @click="copyContent">
            <template #icon><NIcon :component="CopyIcon" size="14" /></template>
          </NButton>
          <NButton text size="tiny" @click="emit('delete', item.id)">
            <template #icon><NIcon :component="DeleteIcon" size="14" color="#E05252" /></template>
          </NButton>
        </NSpace>
      </div>
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
  transition: box-shadow 0.2s;
  cursor: default;
}

.task-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

html.dark .task-card {
  background: #2a2a2a;
  border-color: #444;
}

html.dark .task-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.task-check {
  width: 16px;
  height: 16px;
  border: 2px solid #ccc;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 2px;
}

.task-content {
  flex: 1;
  min-width: 0;
}

.task-title {
  font-size: var(--task-font-size, 14px);
  font-weight: 500;
  color: #333;
  word-break: break-all;
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

.task-thumbnail {
  margin-top: 8px;
  max-width: 200px;
  border-radius: 8px;
  overflow: hidden;
}

.task-thumbnail img {
  width: 100%;
  display: block;
}

.task-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  font-size: calc(var(--task-font-size, 14px) * 0.857);
}

.task-stars {
  display: flex;
  gap: 2px;
}

.task-stars .star {
  color: #ddd;
  cursor: pointer;
  transition: color 0.15s;
}

.task-stars .star.filled {
  color: #FFB800;
}

.task-stars .star:hover {
  color: #FFB800;
}

.task-time {
  color: #888;
  flex: 1;
}

html.dark .task-time {
  color: #999;
}
</style>
