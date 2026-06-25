import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { TimerTask, FixedReminder, TimerDailyStats, TimerSettings } from '../types';
import { DEFAULT_TIMER_TASKS, DEFAULT_FIXED_REMINDERS } from '../types';

function generateId(): string {
  return 'timer_' + Date.now().toString(36) + '_' + Math.random().toString(36).substring(2, 7);
}

function todayStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

const STORAGE_KEY = 'mmy_todolist_timer';
const STATS_KEY = 'mmy_todolist_timer_stats';
const SETTINGS_KEY = 'mmy_todolist_timer_settings';

function loadTasks(): TimerTask[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return DEFAULT_TIMER_TASKS.map(t => ({
    ...t,
    id: generateId(),
    createdAt: Date.now(),
    updatedAt: Date.now(),
  }));
}

function loadReminders(): FixedReminder[] {
  return DEFAULT_FIXED_REMINDERS.map(r => ({ ...r, id: generateId() }));
}

function loadStats(): Record<string, TimerDailyStats> {
  try {
    const raw = localStorage.getItem(STATS_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return {};
}

function loadSettings(): TimerSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { soundEnabled: true, notificationEnabled: true, autoStartNextInLoop: true };
}

export const useTimerStore = defineStore('timer', () => {
  const tasks = ref<TimerTask[]>(loadTasks());
  const reminders = ref<FixedReminder[]>(loadReminders());
  const stats = ref<Record<string, TimerDailyStats>>(loadStats());
  const settings = ref<TimerSettings>(loadSettings());

  const currentTaskId = ref<string | null>(null);
  const isRunning = ref(false);
  const remainingSeconds = ref(0);
  const startedAt = ref<number | null>(null);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  const currentTask = computed(() => {
    if (!currentTaskId.value) return null;
    return tasks.value.find(t => t.id === currentTaskId.value) || null;
  });

  const progress = computed(() => {
    const task = currentTask.value;
    if (!task) return 0;
    return Math.max(0, Math.min(1, remainingSeconds.value / task.duration));
  });

  const todayStats = computed(() => {
    const key = todayStr();
    return stats.value[key] || { date: key, focusSeconds: 0, sessionsCompleted: 0 };
  });

  function persistTasks() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(tasks.value));
  }

  function persistStats() {
    localStorage.setItem(STATS_KEY, JSON.stringify(stats.value));
  }

  function persistSettings() {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings.value));
  }

  function addTask(data: Omit<TimerTask, 'id' | 'createdAt' | 'updatedAt' | 'sortOrder'>) {
    const maxSort = tasks.value.length > 0 ? Math.max(...tasks.value.map(t => t.sortOrder)) : -1;
    const task: TimerTask = {
      ...data,
      id: generateId(),
      sortOrder: maxSort + 1,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    tasks.value.push(task);
    persistTasks();
    return task;
  }

  function updateTask(id: string, updates: Partial<Omit<TimerTask, 'id' | 'createdAt'>>) {
    const idx = tasks.value.findIndex(t => t.id === id);
    if (idx !== -1) {
      tasks.value[idx] = { ...tasks.value[idx], ...updates, updatedAt: Date.now() };
      persistTasks();
    }
  }

  function removeTask(id: string) {
    tasks.value = tasks.value.filter(t => t.id !== id);
    if (currentTaskId.value === id) {
      reset();
    }
    persistTasks();
  }

  function reorder(ids: string[]) {
    ids.forEach((id, idx) => {
      const task = tasks.value.find(t => t.id === id);
      if (task) task.sortOrder = idx;
    });
    tasks.value.sort((a, b) => a.sortOrder - b.sortOrder);
    persistTasks();
  }

  function selectTask(id: string) {
    const task = tasks.value.find(t => t.id === id);
    if (!task) return;
    if (isRunning.value) stop();
    currentTaskId.value = id;
    remainingSeconds.value = task.duration;
    isRunning.value = false;
    startedAt.value = null;
  }

  function start() {
    if (!currentTaskId.value) {
      if (tasks.value.length > 0) {
        selectTask(tasks.value[0].id);
      } else {
        return;
      }
    }
    if (isRunning.value) return;
    isRunning.value = true;
    startedAt.value = Date.now();

    intervalId = setInterval(() => {
      remainingSeconds.value--;
      if (remainingSeconds.value <= 0) {
        remainingSeconds.value = 0;
        _onFinish();
      }
    }, 1000);
  }

  function pause() {
    if (!isRunning.value) return;
    isRunning.value = false;
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function stop() {
    pause();
  }

  function reset() {
    pause();
    if (currentTask.value) {
      remainingSeconds.value = currentTask.value.duration;
    }
    startedAt.value = null;
  }

  function toggle() {
    if (isRunning.value) pause();
    else start();
  }

  function _onFinish() {
    pause();
    const task = currentTask.value;
    if (!task) return;

    if (task.type === 'once') {
      const key = todayStr();
      if (!stats.value[key]) {
        stats.value[key] = { date: key, focusSeconds: 0, sessionsCompleted: 0 };
      }
      stats.value[key].focusSeconds += task.duration;
      stats.value[key].sessionsCompleted += 1;
      persistStats();

      _sendNotification(task);
      isRunning.value = false;
    } else {
      _sendNotification(task);
      if (settings.value.autoStartNextInLoop) {
        setTimeout(() => {
          remainingSeconds.value = task.duration;
          start();
        }, 2000);
      }
    }
  }

  function _sendNotification(task: TimerTask) {
    if (!settings.value.notificationEnabled) return;
    if ('Notification' in window) {
      if (Notification.permission === 'granted') {
        new Notification('⏰ 时间到！', {
          body: `"${task.name}" 计时完成`,
          icon: '/tauri.svg',
        });
      } else if (Notification.permission !== 'denied') {
        Notification.requestPermission().then(perm => {
          if (perm === 'granted') {
            new Notification('⏰ 时间到！', {
              body: `"${task.name}" 计时完成`,
            });
          }
        });
      }
    }
  }

  function toggleReminder(id: string) {
    const r = reminders.value.find(r => r.id === id);
    if (r) {
      r.enabled = !r.enabled;
    }
  }

  function addReminder(data: Omit<FixedReminder, 'id'>) {
    reminders.value.push({ ...data, id: generateId() });
  }

  function removeReminder(id: string) {
    reminders.value = reminders.value.filter(r => r.id !== id);
  }

  function updateSettings(updates: Partial<TimerSettings>) {
    settings.value = { ...settings.value, ...updates };
    persistSettings();
  }

  function startTaskTimer(taskId: string) {
    selectTask(taskId);
    start();
  }

  return {
    tasks,
    reminders,
    stats,
    todayStats,
    settings,
    currentTaskId,
    currentTask,
    isRunning,
    remainingSeconds,
    startedAt,
    progress,
    addTask,
    updateTask,
    removeTask,
    reorder,
    selectTask,
    start,
    pause,
    stop,
    reset,
    toggle,
    startTaskTimer,
    toggleReminder,
    addReminder,
    removeReminder,
    updateSettings,
  };
});
