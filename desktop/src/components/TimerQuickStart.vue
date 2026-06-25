<script setup lang="ts">
import type { TimerTask } from '../types';

defineProps<{
  tasks: TimerTask[];
  currentTaskId: string | null;
}>();

const emit = defineEmits<{
  (e: 'select', taskId: string): void;
}>();

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function getIconBg(color: string): string {
  return `${color}1a`;
}

const emitAdd = () => {
  emit('select', '__add_new__');
};

const handleSelect = (taskId: string) => {
  if (taskId !== '__add_new__') {
    emit('select', taskId);
  }
};
</script>

<template>
  <div class="recent-tasks">
    <div
      v-for="task in tasks"
      :key="task.id"
      class="recent-card"
      :class="{ active: currentTaskId === task.id }"
      @click="handleSelect(task.id)"
    >
      <div class="r-icon" :style="{ background: getIconBg(task.color) }">
        {{ task.icon }}
      </div>
      <div class="r-time">{{ formatTime(task.duration) }}</div>
      <div class="r-name">{{ task.name }}</div>
    </div>
    <div class="recent-card add-new" @click="emitAdd">
      <div class="r-icon">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </div>
      <div class="r-name">新建</div>
    </div>
  </div>
</template>

<style scoped>
.recent-tasks {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding: 6px 4px 8px;
  margin: -6px -4px 0;
  scrollbar-width: none;
  flex-shrink: 0;
}

.recent-tasks::-webkit-scrollbar {
  display: none;
}

.recent-card {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
  width: 96px;
}

.recent-card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

html.dark .recent-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.recent-card.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-light);
}

.r-icon {
  width: 48px;
  height: 48px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  line-height: 1;
  flex-shrink: 0;
}

.r-name {
  font-size: 11px;
  color: var(--text-secondary);
  text-align: center;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.3;
}

.r-time {
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1;
}

.recent-card.add-new {
  border-style: dashed;
  justify-content: center;
  min-height: 130px;
}

.recent-card.add-new .r-icon {
  border: 2px dashed var(--border);
  background: transparent !important;
  font-size: 20px;
  color: var(--text-muted);
  border-radius: 50%;
  width: 44px;
  height: 44px;
}

.recent-card.add-new .r-name {
  color: var(--text-muted);
}

.recent-card.add-new .r-time {
  display: none;
}

.recent-card.add-new:hover {
  border-color: var(--accent);
}

.recent-card.add-new:hover .r-icon {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
