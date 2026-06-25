<script setup lang="ts">
import { computed } from 'vue';
import { useTimerStore } from '../stores/timerStore';
import TimerClock from './TimerClock.vue';
import TimerQuickStart from './TimerQuickStart.vue';
import TimerTaskList from './TimerTaskList.vue';
import FixedReminders from './FixedReminders.vue';
import { useMessage } from 'naive-ui';

const timerStore = useTimerStore();
const message = useMessage();

const totalSeconds = computed(() => timerStore.currentTask?.duration || 0);

function handleToggle() {
  if (timerStore.isRunning) {
    timerStore.pause();
  } else {
    if (!timerStore.currentTaskId && timerStore.tasks.length > 0) {
      timerStore.selectTask(timerStore.tasks[0].id);
    }
    timerStore.start();
  }
}

function handleReset() {
  timerStore.reset();
}

function handleSelectTask(id: string) {
  if (id === '__add_new__') {
    message.info('新建计时任务（开发中）');
    return;
  }
  timerStore.selectTask(id);
}

function handleEditTask(_id: string) {
  message.info('编辑计时任务（开发中）');
}

function handleReorder(ids: string[]) {
  timerStore.reorder(ids);
}

function handleToggleReminder(id: string) {
  if (id === '__add_new__') {
    message.info('添加固定提醒（开发中）');
    return;
  }
  timerStore.toggleReminder(id);
}

const focusMinutes = computed(() => Math.floor(timerStore.todayStats.focusSeconds / 60));
</script>

<template>
  <div class="timer-panel">
    <!-- 时钟表盘 -->
    <TimerClock
      :remaining-seconds="timerStore.remainingSeconds"
      :total-seconds="totalSeconds"
      :task-name="timerStore.currentTask?.name || '选择一个任务开始'"
      :task-type="timerStore.currentTask?.type || 'once'"
      :is-running="timerStore.isRunning"
      @toggle="handleToggle"
      @reset="handleReset"
    />

    <!-- 固定提醒 -->
    <div class="section-title">⏰ 固定提醒</div>
    <FixedReminders
      :reminders="timerStore.reminders"
      @toggle="handleToggleReminder"
    />

    <!-- 快速启动 -->
    <div class="section-title">
      <span>🚀 快速启动</span>
      <span class="link">全部任务 →</span>
    </div>
    <TimerQuickStart
      :tasks="timerStore.tasks"
      :current-task-id="timerStore.currentTaskId"
      @select="handleSelectTask"
    />

    <!-- 全部计时任务 -->
    <div class="section-title">
      <span>📋 全部计时任务</span>
      <span class="link">管理 →</span>
    </div>
    <TimerTaskList
      :tasks="timerStore.tasks"
      :current-task-id="timerStore.currentTaskId"
      :is-running="timerStore.isRunning"
      @select="handleSelectTask"
      @edit="handleEditTask"
      @reorder="handleReorder"
    />

    <!-- 今日统计 -->
    <div class="timer-stats">
      <span>今日专注 <strong>{{ focusMinutes }}</strong> 分钟 · 完成 <strong>{{ timerStore.todayStats.sessionsCompleted }}</strong> 次</span>
    </div>
  </div>
</template>

<style scoped>
.timer-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 4px 0 80px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted, #AAA);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 0 4px 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}
.section-title .link {
  font-size: 11px;
  color: var(--accent, #4A90D9);
  cursor: pointer;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 500;
}
.section-title .link:hover {
  text-decoration: underline;
}

.timer-stats {
  text-align: center;
  font-size: 11px;
  color: var(--text-muted, #AAA);
  padding: 8px;
  border-top: 1px solid var(--border, #E8E8E8);
  margin-top: 8px;
}
.timer-stats strong {
  color: var(--accent, #4A90D9);
  font-weight: 600;
}
</style>
