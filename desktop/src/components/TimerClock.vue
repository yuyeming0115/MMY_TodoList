<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  remainingSeconds: number;
  totalSeconds: number;
  taskName: string;
  taskType: 'once' | 'loop';
  isRunning: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle'): void;
  (e: 'reset'): void;
}>();

const radius = 90;
const circumference = 2 * Math.PI * radius;

const progress = computed(() => {
  if (props.totalSeconds <= 0) return 0;
  return props.remainingSeconds / props.totalSeconds;
});

const dashOffset = computed(() => {
  return circumference * (1 - progress.value);
});

const formattedTime = computed(() => {
  const m = Math.floor(props.remainingSeconds / 60);
  const s = props.remainingSeconds % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
});

const statusText = computed(() => {
  if (props.isRunning) return '计时中...';
  if (props.remainingSeconds < props.totalSeconds) return '已暂停';
  return '准备开始';
});

const typeLabel = computed(() => props.taskType === 'loop' ? '循环' : '常规');
const btnText = computed(() => props.isRunning ? '暂停' : '开始');
</script>

<template>
  <div class="timer-main">
    <div class="timer-status" :class="{ running: isRunning }">
      {{ statusText }}
    </div>
    <div class="clock-wrap">
      <svg class="clock-svg" viewBox="0 0 200 200">
        <circle class="clock-track" cx="100" cy="100" :r="radius" />
        <circle
          class="clock-progress"
          :class="{ running: isRunning }"
          cx="100"
          cy="100"
          :r="radius"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="dashOffset"
        />
      </svg>
      <div class="clock-center">
        <div class="clock-time">{{ formattedTime }}</div>
        <div class="clock-task-name">{{ taskName || '选择一个任务开始' }}</div>
        <div class="clock-type-badge" :class="taskType">{{ typeLabel }}</div>
      </div>
    </div>
    <div class="timer-ctrls">
      <button
        class="timer-btn"
        :class="isRunning ? 'pause' : 'start'"
        @click="emit('toggle')"
      >
        <svg v-if="!isRunning" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <polygon points="5 3 19 12 5 21 5 3" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="4" width="4" height="16" />
          <rect x="14" y="4" width="4" height="16" />
        </svg>
        {{ btnText }}
      </button>
      <button class="timer-btn reset" @click="emit('reset')">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <polyline points="1 4 1 10 7 10" />
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
        </svg>
        重置
      </button>
    </div>
  </div>
</template>

<style scoped>
.timer-main {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 16px 18px;
  background: var(--card-bg);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border);
  position: relative;
  flex-shrink: 0;
}

.timer-status {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 12px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.timer-status.running {
  color: var(--timer-active);
}

.clock-wrap {
  position: relative;
  width: 200px;
  height: 200px;
  margin-bottom: 12px;
}

.clock-svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.clock-track {
  fill: none;
  stroke: var(--border);
  stroke-width: 8;
}

html.dark .clock-track {
  stroke: #3a3a3a;
}

.clock-progress {
  fill: none;
  stroke: var(--accent);
  stroke-width: 8;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.5s ease;
}

.clock-progress.running {
  stroke: var(--timer-active);
}

.clock-center {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.clock-time {
  font-family: 'JetBrains Mono', monospace;
  font-size: 42px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1;
  letter-spacing: -1px;
}

.clock-task-name {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 6px;
  max-width: 160px;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clock-type-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  margin-top: 4px;
  font-weight: 600;
}

.clock-type-badge.once {
  background: var(--accent-light);
  color: var(--accent);
}

.clock-type-badge.loop {
  background: rgba(255, 107, 107, 0.1);
  color: var(--timer-active);
}

.timer-ctrls {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.timer-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 8px 20px;
  border: none;
  border-radius: var(--radius-full);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.timer-btn.start {
  background: var(--timer-active);
  color: white;
  box-shadow: 0 2px 8px rgba(255, 107, 107, 0.3);
}

.timer-btn.start:hover {
  background: #FF5252;
  transform: scale(1.03);
}

.timer-btn.pause {
  background: var(--warning);
  color: white;
}

.timer-btn.pause:hover {
  opacity: 0.9;
  transform: scale(1.03);
}

.timer-btn.reset {
  background: rgba(0, 0, 0, 0.06);
  color: var(--text-secondary);
}

html.dark .timer-btn.reset {
  background: rgba(255, 255, 255, 0.08);
}

.timer-btn.reset:hover {
  background: rgba(0, 0, 0, 0.1);
  color: var(--text-primary);
}
</style>
