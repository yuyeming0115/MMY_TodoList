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
    taskSortMode: 'custom',
    customSortBackup: undefined,
    clipboardSortMode: 'custom',
    clipboardSortBackup: undefined,
    enableClipboardMonitor: true, // 默认启用剪贴板监控
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

  function setTaskSortMode(mode: 'custom' | 'name' | 'updatedAt') {
    update({ taskSortMode: mode });
  }

  function setCustomSortBackup(backup: Record<string, number> | undefined) {
    update({ customSortBackup: backup });
  }

  function setClipboardSortMode(mode: 'custom' | 'name' | 'createdAt') {
    update({ clipboardSortMode: mode });
  }

  function setClipboardSortBackup(backup: Record<string, number> | undefined) {
    update({ clipboardSortBackup: backup });
  }

  function setEnableClipboardMonitor(enable: boolean) {
    update({ enableClipboardMonitor: enable });
    // 同步调用后端启动/停止剪贴板监控
    import('@tauri-apps/api/core').then(({ invoke }) => {
      if (enable) {
        invoke('start_clipboard_monitor_cmd').catch(e => console.error('启动剪贴板监控失败:', e));
      } else {
        invoke('stop_clipboard_monitor_cmd').catch(e => console.error('停止剪贴板监控失败:', e));
      }
    });
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
    setTaskSortMode,
    setCustomSortBackup,
    setClipboardSortMode,
    setClipboardSortBackup,
    setEnableClipboardMonitor,
  };
});