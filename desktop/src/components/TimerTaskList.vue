<script setup lang="ts">
import { computed } from 'vue';
import draggable from 'vuedraggable';
import type { TimerTask } from '../types';

const props = defineProps<{
  tasks: TimerTask[];
  currentTaskId: string | null;
  isRunning: boolean;
}>();

const emit = defineEmits<{
  (e: 'select', taskId: string): void;
  (e: 'edit', taskId: string): void;
  (e: 'reorder', ids: string[]): void;
}>();

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

const localList = computed({
  get: () => props.tasks,
  set: (val: TimerTask[]) => {
    emit('reorder', val.map(t => t.id));
  }
});

const isTaskRunning = (taskId: string) => {
  return props.isRunning && props.currentTaskId === taskId;
};

const handleEdit = (e: Event, taskId: string) => {
  e.stopPropagation();
  emit('edit', taskId);
};
</script>

<template>
  <div class="timer-task-list">
    <draggable
      v-model="localList"
      item-key="id"
      :animation="200"
      handle=".tt-drag"
      ghost-class="ghost"
    >
      <template #item="{ element: task }">
        <div
          class="tt-item"
          :class="{ running: isTaskRunning(task.id) }"
          @click="emit('select', task.id)"
        >
          <span class="tt-drag">⋮⋮</span>
          <span class="tt-color" :style="{ background: task.color }"></span>
          <div class="tt-info">
            <div class="tt-name">{{ task.icon }} {{ task.name }}</div>
            <div class="tt-meta">
              <span v-if="task.type === 'loop'" class="tt-loop">🔄 循环</span>
              <span v-else>常规</span>
            </div>
          </div>
          <span class="tt-time">{{ formatTime(task.duration) }}</span>
          <div class="tt-actions">
            <button class="op-btn" @click="handleEdit($event, task.id)" title="编辑">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
              </svg>
            </button>
          </div>
        </div>
      </template>
    </draggable>
  </div>
</template>

<style scoped>
.timer-task-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tt-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
}

.tt-item:hover {
  border-color: var(--accent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
}

html.dark .tt-item:hover {
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.tt-item.running {
  border-color: var(--timer-active);
  background: rgba(255, 107, 107, 0.04);
}

.ghost {
  opacity: 0.5;
  background: var(--accent-light);
  border-style: dashed;
}

.tt-drag {
  color: var(--text-muted);
  cursor: grab;
  display: flex;
  align-items: center;
  font-size: 14px;
  user-select: none;
}

.tt-drag:active {
  cursor: grabbing;
}

.tt-color {
  width: 4px;
  height: 24px;
  border-radius: 2px;
  flex-shrink: 0;
}

.tt-info {
  flex: 1;
  min-width: 0;
}

.tt-name {
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tt-meta {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.tt-loop {
  color: var(--timer-active);
}

.tt-time {
  font-family: 'JetBrains Mono', monospace;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.tt-item.running .tt-time {
  color: var(--timer-active);
}

.tt-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
}

.tt-item:hover .tt-actions {
  opacity: 1;
}

.op-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  padding: 0;
}

.op-btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: var(--text-primary);
}

html.dark .op-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}
</style>
