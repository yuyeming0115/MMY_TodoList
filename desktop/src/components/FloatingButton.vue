<script setup lang="ts">
defineProps<{
  panel: 'tasks' | 'timer' | 'clipboard';
}>();
const emit = defineEmits<{
  (e: 'click'): void;
}>();
</script>

<template>
  <button class="fab" :class="{ 'fab-timer': panel === 'timer' }" @click="emit('click')" :title="panel === 'timer' ? '添加计时任务' : '添加任务'">
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
      <line x1="12" y1="5" x2="12" y2="19"/>
      <line x1="5" y1="12" x2="19" y2="12"/>
    </svg>
  </button>
</template>

<style scoped>
.fab {
  position: absolute;
  bottom: 16px;
  right: 16px;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--accent);
  color: white;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  box-shadow: 0 4px 16px rgba(74,144,217,0.4);
  transition: all 0.2s;
  z-index: 100;
}
.fab:hover {
  background: var(--accent-hover);
  transform: scale(1.08) rotate(90deg);
  box-shadow: 0 6px 20px rgba(74,144,217,0.5);
}
.fab-timer {
  background: var(--timer-active);
  box-shadow: 0 4px 16px rgba(255,107,107,0.4);
}
.fab-timer:hover {
  background: #FF5252;
  box-shadow: 0 6px 20px rgba(255,107,107,0.5);
}
</style>
