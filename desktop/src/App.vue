<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme } from 'naive-ui';
import { computed, watch } from 'vue';
import { useSettingsStore } from './stores/settingsStore';
import Home from './views/Home.vue';

const settingsStore = useSettingsStore();

const themeOverrides = {
  common: {
    primaryColor: '#4A90D9',
    primaryColorHover: '#5BA4F5',
    borderRadius: '8px'
  }
};

const isDark = computed(() => {
  if (settingsStore.settings.themeMode === 'dark') return true;
  if (settingsStore.settings.themeMode === 'light') return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
});

watch(isDark, (val) => {
  document.documentElement.classList.toggle('dark', val);
}, { immediate: true });
</script>

<template>
  <NConfigProvider :theme="isDark ? darkTheme : null" :themeOverrides="themeOverrides">
    <NMessageProvider :container-style="{ top: '48px !important' }">
      <NDialogProvider>
        <Home />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
