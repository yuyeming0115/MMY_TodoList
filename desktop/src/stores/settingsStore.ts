import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppSettings } from '../types';
import { getSettings, updateSettings } from '../utils/db';

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    themeMode: 'system',
    language: 'zh',
    hideCompletedTasks: false,
    launchAtStartup: false
  });

  async function load() {
    settings.value = await getSettings();
  }

  async function update(newSettings: Partial<AppSettings>) {
    settings.value = { ...settings.value, ...newSettings };
    await updateSettings(settings.value);
  }

  function setTheme(mode: 'system' | 'light' | 'dark') {
    update({ themeMode: mode });
  }

  function setLanguage(lang: 'zh' | 'en') {
    update({ language: lang });
  }

  function setHideCompleted(hide: boolean) {
    update({ hideCompletedTasks: hide });
  }

  function setLaunchAtStartup(enable: boolean) {
    update({ launchAtStartup: enable });
  }

  function setWindowSize(width: number, height: number) {
    update({ windowWidth: width, windowHeight: height });
  }

  return {
    settings,
    load,
    update,
    setTheme,
    setLanguage,
    setHideCompleted,
    setLaunchAtStartup,
    setWindowSize
  };
});