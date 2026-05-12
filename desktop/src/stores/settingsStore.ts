import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppSettings } from '../types';
import { getSettings, updateSettings } from '../utils/db';

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    themeMode: 'system',
    language: 'zh',
    hideCompletedTasks: false,
    launchAtStartup: false,
    fontSize: 14,
    fontFamily: '',
    clipboardViewMode: 'normal',
    clipboardStackGap: 64,
    taskViewMode: 'normal',
    globalShortcut: undefined,
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

  function setWindowPosition(x: number, y: number) {
    update({ windowX: x, windowY: y });
  }

  function setFontSize(size: number) {
    update({ fontSize: size });
  }

  function setFontFamily(family: string) {
    update({ fontFamily: family });
  }

  function setClipboardViewMode(mode: 'normal' | 'stacked') {
    update({ clipboardViewMode: mode });
  }

  function setClipboardStackGap(gap: number) {
    update({ clipboardStackGap: gap });
  }

  function setTaskViewMode(mode: 'normal' | 'stacked') {
    update({ taskViewMode: mode });
  }

  function setGlobalShortcut(shortcut: string | undefined) {
    update({ globalShortcut: shortcut });
  }

  return {
    settings,
    load,
    update,
    setTheme,
    setLanguage,
    setHideCompleted,
    setLaunchAtStartup,
    setWindowSize,
    setWindowPosition,
    setFontSize,
    setFontFamily,
    setClipboardViewMode,
    setClipboardStackGap,
    setTaskViewMode,
    setGlobalShortcut,
  };
});