<script setup lang="ts">
import type { FixedReminder } from '../types';

defineProps<{
  reminders: FixedReminder[];
}>();

const emit = defineEmits<{
  (e: 'toggle', id: string): void;
}>();

const handleAdd = () => {
  emit('toggle', '__add_new__');
};

const handleToggle = (id: string) => {
  if (id !== '__add_new__') {
    emit('toggle', id);
  }
};
</script>

<template>
  <div class="fixed-reminders">
    <div
      v-for="reminder in reminders"
      :key="reminder.id"
      class="fr-chip"
      :class="{ active: reminder.enabled }"
      @click="handleToggle(reminder.id)"
    >
      {{ reminder.icon }}
      <span class="fr-time">{{ reminder.time }}</span>
      {{ reminder.name }}
    </div>
    <div class="fr-chip add-btn" @click="handleAdd">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
      添加
    </div>
  </div>
</template>

<style scoped>
.fixed-reminders {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  flex-shrink: 0;
}

.fr-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--text-secondary);
}

.fr-chip:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

.fr-chip.active {
  background: var(--accent-light);
  border-color: var(--accent);
  color: var(--accent);
}

.fr-time {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 600;
  font-size: 11px;
}

.add-btn {
  border-style: dashed;
  color: var(--text-muted);
}

.add-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-light);
}
</style>
