<script setup lang="ts">
import {
  NForm, NFormItem, NSelect, NSwitch, NSpace,
  NButton, NDivider, NIcon, NText
} from 'naive-ui';
import {
  DownloadOutline as ExportIcon, CloudUploadOutline as ImportIcon,
  ArrowBackOutline as BackIcon
} from '@vicons/ionicons5';
import { useSettingsStore } from '../stores/settingsStore';
import { useMessage } from 'naive-ui';
import { exportData, importData } from '../utils/db';
import type { ExportData } from '../types';

const emit = defineEmits<{
  (e: 'back'): void;
}>();

const settingsStore = useSettingsStore();
const message = useMessage();

// 主题选项
const themeOptions = [
  { label: '跟随系统', value: 'system' },
  { label: '亮色', value: 'light' },
  { label: '暗色', value: 'dark' }
];

// 语言选项
const languageOptions = [
  { label: '简体中文', value: 'zh' },
  { label: 'English', value: 'en' }
];

// 导出数据
async function handleExport() {
  try {
    const data = await exportData();
    const json = JSON.stringify(data, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    // 创建下载链接
    const a = document.createElement('a');
    a.href = url;
    a.download = `mmy_todo_${Date.now()}.mmytodo`;
    a.click();
    URL.revokeObjectURL(url);

    message.success('导出成功');
  } catch (e) {
    message.error('导出失败');
    console.error(e);
  }
}

// 导入数据
async function handleImport() {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.mmytodo,.json';

  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      const data = JSON.parse(text) as ExportData;

      // 验证格式
      if (!data.version || !data.categories || !data.tasks) {
        message.error('无效的备份文件');
        return;
      }

      await importData(data);
      message.success('导入成功，数据已更新');

      // 重新加载
      window.location.reload();
    } catch (e) {
      message.error('导入失败');
      console.error(e);
    }
  };

  input.click();
}

function goBack() {
  emit('back');
}
</script>

<template>
  <div class="page">
    <!-- 页面头部 -->
    <header class="page-header">
      <NButton text size="large" @click="goBack">
        <template #icon>
          <NIcon :component="BackIcon" size="20" />
        </template>
      </NButton>
      <span class="page-title">设置</span>
    </header>

    <!-- 页面内容 -->
    <div class="page-content">
      <NForm label-placement="left" label-width="100">
        <NFormItem label="主题">
          <NSelect
            :value="settingsStore.settings.themeMode"
            :options="themeOptions"
            @update:value="(v: string) => settingsStore.setTheme(v as any)"
          />
        </NFormItem>

        <NFormItem label="语言">
          <NSelect
            :value="settingsStore.settings.language"
            :options="languageOptions"
            @update:value="(v: string) => settingsStore.setLanguage(v as any)"
          />
        </NFormItem>

        <NFormItem label="隐藏已完成">
          <NSwitch
            :value="settingsStore.settings.hideCompletedTasks"
            @update:value="(v: boolean) => settingsStore.setHideCompleted(v)"
          />
        </NFormItem>

        <NFormItem label="开机启动">
          <NSwitch
            :value="settingsStore.settings.launchAtStartup"
            @update:value="(v: boolean) => settingsStore.setLaunchAtStartup(v)"
          />
        </NFormItem>
      </NForm>

      <NDivider />

      <div class="data-section">
        <NText depth="2" style="font-weight: 500">数据管理</NText>
        <NSpace :size="12" style="margin-top: 12px">
          <NButton @click="handleExport">
            <template #icon>
              <NIcon :component="ExportIcon" />
            </template>
            导出数据
          </NButton>
          <NButton @click="handleImport">
            <template #icon>
              <NIcon :component="ImportIcon" />
            </template>
            导入数据
          </NButton>
        </NSpace>
        <NText depth="3" style="font-size: 12px; margin-top: 8px; display: block">
          导出文件格式：.mmytodo（JSON）
        </NText>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #f5f5f5;
}

html.dark .page {
  background: #1a1a1a;
}

.page-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid #e0e0e0;
  background: #fff;
  flex-shrink: 0;
}

html.dark .page-header {
  background: #242424;
  border-bottom-color: #333;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: #333;
}

html.dark .page-title {
  color: #e0e0e0;
}

.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  scrollbar-width: thin;
  scrollbar-color: rgba(100, 100, 100, 0.2) transparent;
}

html.dark .page-content {
  scrollbar-color: rgba(80, 80, 80, 0.4) transparent;
}

.page-content::-webkit-scrollbar {
  width: 4px;
}

.page-content::-webkit-scrollbar-track {
  background: transparent;
}

.page-content::-webkit-scrollbar-thumb {
  background: rgba(100, 100, 100, 0.2);
  border-radius: 10px;
}

html.dark .page-content::-webkit-scrollbar-thumb {
  background: rgba(80, 80, 80, 0.4);
}

.data-section {
  margin-top: 8px;
}
</style>