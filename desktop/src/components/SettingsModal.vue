<script setup lang="ts">
import {
  NModal, NForm, NFormItem, NSelect, NSwitch, NSpace,
  NButton, NDivider, NIcon, NText
} from 'naive-ui';
import {
  DownloadOutline as ExportIcon, CloudUploadOutline as ImportIcon
} from '@vicons/ionicons5';
import { useSettingsStore } from '../stores/settingsStore';
import { useMessage } from 'naive-ui';
import { exportData, importData } from '../utils/db';
import type { ExportData } from '../types';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: 'close'): void;
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

function handleClose() {
  emit('close');
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="设置"
    style="width: 360px"
    :mask-closable="true"
    @close="handleClose"
  >
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

    <NSpace vertical :size="12">
      <NText depth="2">数据管理</NText>
      <NSpace :size="12">
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
      <NText depth="3" style="font-size: 12px">
        导出文件格式：.mmytodo（JSON）
      </NText>
    </NSpace>
  </NModal>
</template>