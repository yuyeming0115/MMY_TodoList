<script setup lang="ts">
import {
  NForm, NFormItem, NSelect, NSwitch, NSpace,
  NButton, NDivider, NIcon, NText, NSlider
} from 'naive-ui';
import {
  DownloadOutline as ExportIcon, CloudUploadOutline as ImportIcon,
  ArrowBackOutline as BackIcon, Star as StarIcon
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

// 字体选项
const fontOptions = [
  { label: '系统默认', value: '' },
  { label: '阿里妈妈东方大楷', value: 'AlimamaDongfangDakai' },
  { label: '抖音美好体', value: 'DouyinMeihaoTi' },
];

// 导出数据
async function handleExport() {
  try {
    const data = await exportData();
    const json = JSON.stringify(data, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

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

      if (!data.version || !data.categories || !data.tasks) {
        message.error('无效的备份文件');
        return;
      }

      await importData(data);
      message.success('导入成功，数据已更新');

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

      <!-- 字体设置 -->
      <div class="font-section">
        <NText depth="2" style="font-weight: 500">字体设置</NText>

        <NForm label-placement="left" label-width="100" style="margin-top: 12px">
          <NFormItem label="字体大小">
            <div class="font-size-control">
              <NSlider
                :value="settingsStore.settings.fontSize"
                :min="14"
                :max="36"
                :step="1"
                @update:value="(v: number) => settingsStore.setFontSize(v)"
              />
              <span class="font-size-value">{{ settingsStore.settings.fontSize }}px</span>
            </div>
          </NFormItem>

          <NFormItem label="字体">
            <NSelect
              :value="settingsStore.settings.fontFamily"
              :options="fontOptions"
              @update:value="(v: string) => settingsStore.setFontFamily(v)"
            />
          </NFormItem>
        </NForm>

        <!-- 实时预览卡片 -->
        <div class="preview-label">
          <NText depth="3" style="font-size: 12px">预览效果</NText>
        </div>
        <div class="preview-card">
          <div class="preview-check" />
          <div class="preview-content">
            <div class="preview-title">这是一段预览任务标题</div>
            <div class="preview-desc">描述文字预览...</div>
            <div class="preview-meta">
              <span class="preview-stars">
                <NIcon :component="StarIcon" size="14" class="star filled" />
                <NIcon :component="StarIcon" size="14" class="star filled" />
                <NIcon :component="StarIcon" size="14" class="star" />
              </span>
              <span class="preview-time">开始 截止</span>
            </div>
          </div>
        </div>
      </div>

      <NDivider />

      <!-- 数据管理 -->
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

/* 字体设置区域 */
.font-section {
  margin-top: 8px;
}

.font-size-control {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.font-size-value {
  min-width: 40px;
  font-weight: 600;
  color: #4A90D9;
  font-size: 14px;
  text-align: right;
}

/* 预览卡片 */
.preview-label {
  margin-top: 8px;
  margin-bottom: 8px;
}

.preview-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  border-left: 3px solid #4A90D9;
  font-size: var(--task-font-size, 14px);
  font-family: var(--task-font-family, inherit);
}

html.dark .preview-card {
  background: #2a2a2a;
  border-color: #444;
}

.preview-check {
  width: 16px;
  height: 16px;
  border: 2px solid #ccc;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 2px;
  background: #e8f5e9;
  border-color: #28C840;
}

html.dark .preview-check {
  background: rgba(40, 200, 64, 0.15);
  border-color: rgba(40, 200, 64, 0.4);
}

.preview-content {
  flex: 1;
  min-width: 0;
}

.preview-title {
  font-size: var(--task-font-size, 14px);
  font-weight: 500;
  color: #333;
}

html.dark .preview-title {
  color: #e0e0e0;
}

.preview-desc {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #888;
  margin-top: 4px;
}

html.dark .preview-desc {
  color: #999;
}

.preview-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 4px;
  font-size: calc(var(--task-font-size, 14px) * 0.857);
}

.preview-stars {
  display: flex;
  gap: 2px;
}

.preview-stars .star {
  color: #ddd;
}

.preview-stars .star.filled {
  color: #FFB800;
}

.preview-time {
  color: #888;
  display: flex;
  gap: 6px;
}

html.dark .preview-time {
  color: #999;
}
</style>
