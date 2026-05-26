<script setup lang="ts">
import {
  NForm, NFormItem, NSelect, NSwitch, NSpace,
  NButton, NDivider, NIcon, NText, NSlider, NPopconfirm, NModal, NCheckbox
} from 'naive-ui';
import {
  DownloadOutline as ExportIcon, CloudUploadOutline as ImportIcon,
  ArrowBackOutline as BackIcon, Star as StarIcon, CloudOutline as BackupIcon,
  TrashOutline as TrashIcon, RefreshOutline as RestoreIcon, TimeOutline as TimeIcon,
  CloseOutline as CloseIcon, SaveOutline as SaveIcon,
} from '@vicons/ionicons5';
import { useSettingsStore } from '../stores/settingsStore';
import { useI18n } from '../composables/useI18n';
import { useMessage } from 'naive-ui';
import { exportData, importData, getBackupSettings, updateBackupSettings, createBackupWithType, listBackups, restoreBackup, deleteBackup, previewBackup, restoreBackupWithOptions, updateGlobalShortcut } from '../utils/db';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import type { ExportData, BackupSettings, BackupInfo, BackupPreview, RestoreOptions } from '../types';
import { ref, onMounted, computed } from 'vue';

const emit = defineEmits<{
  (e: 'back'): void;
}>();

const settingsStore = useSettingsStore();
const message = useMessage();
const { t } = useI18n();

// 字体选项
const fontOptions = computed(() => [
  { label: t('settings.fontDefault'), value: '' },
  { label: '阿里妈妈东方大楷', value: 'AlimamaDongfangDakai' },
  { label: '抖音美好体', value: 'DouyinMeihaoTi' },
]);

// 备份设置
const backupSettings = ref<BackupSettings>({
  backupOnClose: true,
  backupHourly: false,
  retentionDays: 7,
  defaultBackupType: 'quick',
});

// 备份类型选项
const backupTypeOptions = computed(() => [
  { label: t('settings.backupTypeQuick'), value: 'quick' },
  { label: t('settings.backupTypeFull'), value: 'full' },
]);

// 备份列表
const backups = ref<BackupInfo[]>([]);

// 加载备份设置和备份列表
async function loadBackupData() {
  try {
    backupSettings.value = await getBackupSettings();
    backups.value = await listBackups();
  } catch (e) {
    console.error('加载备份设置失败:', e);
  }
}

// 更新备份设置
async function updateBackup(key: keyof BackupSettings, value: boolean | number | string) {
  (backupSettings.value as any)[key] = value;
  try {
    await updateBackupSettings(backupSettings.value);
    message.success(t('messages.settingsSaved'));
  } catch (e) {
    message.error(t('messages.backupFailed'));
    console.error(e);
  }
}

// 立即快速备份
async function handleBackupQuick() {
  try {
    const filename = await createBackupWithType('quick');
    message.success(t('messages.backupSuccess', { filename }));
    await loadBackupData();
  } catch (e) {
    message.error(t('messages.backupFailed'));
    console.error(e);
  }
}

// 立即完整备份
async function handleBackupFull() {
  try {
    const filename = await createBackupWithType('full');
    message.success(t('messages.backupSuccess', { filename }));
    await loadBackupData();
  } catch (e) {
    message.error(t('messages.backupFailed'));
    console.error(e);
  }
}

// 恢复预览弹窗
const showRestoreModal = ref(false);
const restorePreview = ref<BackupPreview | null>(null);
const restoreOptions = ref<RestoreOptions>({
  overwrite: true,
  restoreTasks: true,
  restoreClipboard: true,
  restoreSettings: true,
});
const restoringFilename = ref('');

// 打开恢复预览弹窗
async function openRestorePreview(filename: string) {
  restoringFilename.value = filename;
  try {
    restorePreview.value = await previewBackup(filename);
    restoreOptions.value = {
      overwrite: true,
      restoreTasks: true,
      restoreClipboard: true,
      restoreSettings: true,
    };
    showRestoreModal.value = true;
  } catch (e) {
    message.error(t('messages.previewFailed'));
    console.error(e);
  }
}

// 执行选择性恢复
async function doRestoreWithOptions() {
  try {
    await restoreBackupWithOptions(restoringFilename.value, restoreOptions.value);
    message.success(t('messages.restoreSuccess'));
    showRestoreModal.value = false;
    setTimeout(() => window.location.reload(), 1000);
  } catch (e) {
    message.error(t('messages.restoreFailed'));
    console.error(e);
  }
}

// 快速恢复（覆盖所有）
async function quickRestore(filename: string) {
  try {
    await restoreBackup(filename);
    message.success(t('messages.restoreSuccess'));
    setTimeout(() => window.location.reload(), 1000);
  } catch (e) {
    message.error(t('messages.restoreFailed'));
    console.error(e);
  }
}

// 删除备份
async function handleDeleteBackup(filename: string) {
  try {
    await deleteBackup(filename);
    message.success(t('messages.deleteSuccess'));
    await loadBackupData();
  } catch (e) {
    message.error(t('messages.deleteFailed'));
    console.error(e);
  }
}

// 格式化备份时间
function formatBackupTime(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

// 格式化文件大小
function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

// 格式化备份类型显示
function formatBackupType(type: string): string {
  return type === 'full' ? t('settings.backupTypeFull') : t('settings.backupTypeQuick');
}

// 快捷键设置
const isCapturing = ref(false);
const currentShortcut = ref<string | undefined>(undefined);
const shortcutError = ref<string | null>(null);

// 初始化快捷键
onMounted(() => {
  loadBackupData();
  currentShortcut.value = settingsStore.settings.globalShortcut;
});

// 开始捕获快捷键
function startCapturing() {
  isCapturing.value = true;
  shortcutError.value = null;
}

// 捕获按键
function captureShortcut(e: KeyboardEvent) {
  if (!isCapturing.value) return;

  e.preventDefault();
  e.stopPropagation();

  // Escape 取消
  if (e.key === 'Escape') {
    isCapturing.value = false;
    return;
  }

  // 收集修饰键
  const modifiers: string[] = [];
  if (e.ctrlKey) modifiers.push('Ctrl');
  if (e.altKey) modifiers.push('Alt');
  if (e.shiftKey) modifiers.push('Shift');

  // 获取主按键
  let key = e.key;
  if (key.length === 1) {
    key = key.toUpperCase();
  } else if (key.startsWith('F') && /^\d+$/.test(key.slice(1))) {
    // F1-F24 保持原样
  } else if (['Enter', 'Space', 'Backspace', 'Tab', 'Insert', 'Delete', 'Home', 'End', 'PageUp', 'PageDown'].includes(key)) {
    // 特殊键保持原样
  } else {
    // 其他键忽略（如箭头键等）
    return;
  }

  // 组合：最多3个按键（2个修饰键 + 1个主键）
  const allKeys = [...modifiers, key];
  if (allKeys.length > 3) {
    allKeys.splice(2, allKeys.length - 3);
  }

  // 至少要有主键，单键必须是 F1-F24
  if (modifiers.length === 0 && key.length > 1 && !key.startsWith('F') && !['Enter', 'Space'].includes(key)) {
    return;
  }

  const shortcut = allKeys.join('+');
  currentShortcut.value = shortcut;
  isCapturing.value = false;

  // 保存并注册
  saveShortcut(shortcut);
}

// 保存快捷键
async function saveShortcut(shortcut: string) {
  try {
    await updateGlobalShortcut(shortcut);
    settingsStore.setGlobalShortcut(shortcut);
    message.success(t('settings.shortcutRegistered'));
    shortcutError.value = null;
  } catch (e: any) {
    const errorMsg = e?.message || e?.toString() || '';
    shortcutError.value = t('settings.shortcutFailed');
    message.error(t('settings.shortcutFailed'));
    // 注册失败时清除设置，让用户知道需要重试
    currentShortcut.value = undefined;
    console.error('快捷键注册失败:', errorMsg);
  }
}

// 清除快捷键
async function clearShortcut() {
  try {
    await updateGlobalShortcut(null);
    settingsStore.setGlobalShortcut(undefined);
    currentShortcut.value = undefined;
    message.success(t('settings.shortcutClear'));
    shortcutError.value = null;
  } catch (e) {
    message.error(t('messages.deleteFailed'));
    console.error(e);
  }
}

// 格式化快捷键显示
function formatShortcut(shortcut: string): string {
  return shortcut.split('+').join(' + ');
}

// 导出数据
async function handleExport() {
  try {
    const filePath = await saveDialog({
      filters: [{ name: 'MMY Todo 备份', extensions: ['mmytodo'] }],
      defaultPath: `mmy_todo_${new Date().toISOString().slice(0, 10)}.mmytodo`,
    });

    if (!filePath) return;

    const data = await exportData();
    const json = JSON.stringify(data, null, 2);
    const bytes = new TextEncoder().encode(json);

    // writeFile 需要路径字符串
    await writeFile(filePath.toString(), bytes);

    message.success(t('messages.exportSuccess', { path: filePath }));
  } catch (e) {
    message.error(t('messages.exportFailed'));
    console.error('导出失败:', e);
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
        message.error(t('messages.invalidFile'));
        return;
      }

      await importData(data);
      message.success(t('messages.importSuccess'));

      window.location.reload();
    } catch (e) {
      message.error(t('messages.importFailed'));
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
      <span class="page-title">{{ t('settings.title') }}</span>
    </header>

    <!-- 页面内容 -->
    <div class="page-content">
      <!-- 基础设置：两个开关一行 -->
      <div class="settings-row">
        <div class="setting-item">
          <span class="setting-label">{{ t('settings.hideCompleted') }}</span>
          <NSwitch
            :value="settingsStore.settings.hideCompletedTasks"
            @update:value="(v: boolean) => settingsStore.setHideCompleted(v)"
          />
        </div>
        <div class="setting-item">
          <span class="setting-label">{{ t('settings.launchAtStartup') }}</span>
          <NSwitch
            :value="settingsStore.settings.launchAtStartup"
            @update:value="(v: boolean) => settingsStore.setLaunchAtStartup(v)"
          />
        </div>
      </div>

      <NDivider />

      <!-- 快捷键设置 -->
      <div class="shortcut-section">
        <NText depth="2" style="font-weight: 500">{{ t('settings.shortcut') }}</NText>

        <div class="shortcut-input-wrapper" style="margin-top: 12px">
          <div
            class="shortcut-input"
            :class="{ active: isCapturing, error: shortcutError }"
            tabindex="0"
            @click="startCapturing"
            @keydown="captureShortcut"
            @blur="isCapturing = false"
          >
            <span v-if="currentShortcut" class="shortcut-display">{{ formatShortcut(currentShortcut) }}</span>
            <span v-else class="placeholder">{{ isCapturing ? t('settings.pressShortcut') : t('settings.clickToSet') }}</span>
          </div>
          <NButton v-if="currentShortcut" text size="small" @click="clearShortcut" style="margin-left: 8px">
            <template #icon>
              <NIcon :component="CloseIcon" size="16" />
            </template>
          </NButton>
        </div>

        <NText depth="3" style="font-size: 12px; margin-top: 8px; display: block; line-height: 1.6">
          {{ t('settings.shortcutHint') }}
        </NText>
        <NText depth="3" style="font-size: 12px; display: block; line-height: 1.6">
          {{ t('settings.shortcutHintFull') }}
        </NText>
        <NText v-if="shortcutError" type="error" style="font-size: 12px; display: block; margin-top: 4px">
          {{ shortcutError }}
        </NText>
      </div>

      <NDivider />

      <!-- 自动备份设置 -->
      <div class="backup-section">
        <NText depth="2" style="font-weight: 500">{{ t('settings.autoBackup') }}</NText>

        <!-- 两个备份开关一行 -->
        <div class="settings-row backup-switches">
          <div class="setting-item">
            <span class="setting-label">{{ t('settings.backupOnClose') }}</span>
            <NSwitch
              :value="backupSettings.backupOnClose"
              @update:value="(v: boolean) => updateBackup('backupOnClose', v)"
            />
          </div>
          <div class="setting-item">
            <span class="setting-label">{{ t('settings.backupHourly') }}</span>
            <NSwitch
              :value="backupSettings.backupHourly"
              @update:value="(v: boolean) => updateBackup('backupHourly', v)"
            />
          </div>
        </div>

        <NForm label-placement="left" label-width="120" style="margin-top: 8px">
          <NFormItem :label="t('settings.retentionDays')">
            <NSlider
              :value="backupSettings.retentionDays"
              :min="1"
              :max="30"
              :step="1"
              @update:value="(v: number) => updateBackup('retentionDays', v)"
              style="width: 150px"
            />
            <span class="retention-value">{{ backupSettings.retentionDays }} {{ t('settings.daysUnit') }}</span>
          </NFormItem>
          <NFormItem :label="t('settings.defaultBackupType')">
            <NSelect
              :value="backupSettings.defaultBackupType"
              :options="backupTypeOptions"
              @update:value="(v: string) => updateBackup('defaultBackupType', v)"
              style="width: 120px"
            />
          </NFormItem>
        </NForm>

        <!-- 立即备份按钮组 -->
        <div class="backup-buttons">
          <NButton type="primary" @click="handleBackupQuick">
            <template #icon>
              <NIcon :component="SaveIcon" />
            </template>
            {{ t('settings.backupQuick') }}
          </NButton>
          <NButton @click="handleBackupFull">
            <template #icon>
              <NIcon :component="BackupIcon" />
            </template>
            {{ t('settings.backupFull') }}
          </NButton>
        </div>

        <NText depth="3" style="font-size: 12px; margin-top: 8px; display: block; line-height: 1.6">
          {{ t('settings.backupLocation') }}
        </NText>
      </div>

      <!-- 备份列表（最多显示7个） -->
      <div v-if="backups.length > 0" class="backup-list">
        <NText depth="2" style="font-weight: 500; margin-bottom: 8px; display: block">{{ t('settings.backupHistory') }}</NText>
        <div class="backup-items">
          <div v-for="backup in backups.slice(0, 7)" :key="backup.filename" class="backup-item">
            <div class="backup-info">
              <NIcon :component="TimeIcon" size="16" style="color: #4A90D9" />
              <span class="backup-time">{{ formatBackupTime(backup.createdAt) }}</span>
              <span class="backup-type-tag" :class="backup.backupType">{{ formatBackupType(backup.backupType) }}</span>
              <span class="backup-size">{{ formatFileSize(backup.sizeBytes) }}</span>
            </div>
            <div class="backup-actions">
              <button class="backup-btn restore" :title="t('settings.restore')" @click="openRestorePreview(backup.filename)">
                <NIcon :component="RestoreIcon" size="14" />
              </button>
              <button class="backup-btn quick-restore" :title="t('settings.quickRestore')" @click="quickRestore(backup.filename)">
                <NIcon :component="RestoreIcon" size="14" />
              </button>
              <NPopconfirm @positive-click="handleDeleteBackup(backup.filename)">
                <template #trigger>
                  <button class="backup-btn delete" :title="t('settings.deleteBackup')">
                    <NIcon :component="TrashIcon" size="14" />
                  </button>
                </template>
                {{ t('settings.deleteConfirm') }}
              </NPopconfirm>
            </div>
          </div>
        </div>
      </div>

      <NDivider />

      <!-- 字体设置 -->
      <div class="font-section">
        <NText depth="2" style="font-weight: 500">{{ t('settings.fontSettings') }}</NText>

        <NForm label-placement="left" label-width="100" style="margin-top: 12px">
          <NFormItem :label="t('settings.fontSize')">
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

          <NFormItem :label="t('settings.fontFamily')">
            <NSelect
              :value="settingsStore.settings.fontFamily"
              :options="fontOptions"
              @update:value="(v: string) => settingsStore.setFontFamily(v)"
            />
          </NFormItem>
        </NForm>

        <!-- 实时预览卡片 -->
        <div class="preview-label">
          <NText depth="3" style="font-size: 12px">{{ t('settings.preview') }}</NText>
        </div>
        <div class="preview-card">
          <div class="preview-check" />
          <div class="preview-content">
            <div class="preview-title">{{ t('settings.previewTitle') }}</div>
            <div class="preview-desc">{{ t('settings.previewDesc') }}</div>
            <div class="preview-meta">
              <span class="preview-stars">
                <NIcon :component="StarIcon" size="14" class="star filled" />
                <NIcon :component="StarIcon" size="14" class="star filled" />
                <NIcon :component="StarIcon" size="14" class="star" />
              </span>
              <span class="preview-time">{{ t('settings.previewStart') }} {{ t('settings.previewDue') }}</span>
            </div>
          </div>
        </div>
      </div>

      <NDivider />

      <!-- 层叠间距设置 -->
      <div class="stack-gap-section">
        <NText depth="2" style="font-weight: 500">{{ t('settings.stackGap') }}</NText>

        <NForm label-placement="left" label-width="120" style="margin-top: 12px">
          <NFormItem :label="t('settings.cardGap')">
            <div class="gap-control">
              <NSlider
                :value="settingsStore.settings.clipboardStackGap"
                :min="20"
                :max="120"
                :step="1"
                @update:value="(v: number) => settingsStore.setClipboardStackGap(v)"
                style="width: 150px"
              />
              <span class="gap-value">{{ settingsStore.settings.clipboardStackGap }}px</span>
            </div>
          </NFormItem>
        </NForm>

        <NText depth="3" style="font-size: 12px; margin-top: 4px; display: block">
          {{ t('settings.stackApply') }}
        </NText>

        <!-- 层叠预览 -->
        <div class="preview-label">
          <NText depth="3" style="font-size: 12px">{{ t('settings.stackPreview') }}</NText>
        </div>
        <div class="stack-preview">
          <div
            v-for="i in 5"
            :key="i"
            class="preview-card-stack"
            :style="{ marginBottom: `${-80 + settingsStore.settings.clipboardStackGap}px`, transform: i % 2 === 0 ? 'translateX(-3px)' : 'translateX(3px)' }"
          >
            <div class="preview-check-stack" />
            <div class="preview-content-stack">
              <div class="preview-title-stack">{{ t('settings.clipboardItem') }} {{ i }}</div>
              <div class="preview-desc-stack">{{ t('settings.contentPreview') }}</div>
            </div>
          </div>
        </div>
      </div>

      <NDivider />

      <!-- 数据管理 -->
      <div class="data-section">
        <NText depth="2" style="font-weight: 500">{{ t('settings.dataManagement') }}</NText>
        <NSpace :size="12" style="margin-top: 12px">
          <NButton @click="handleExport">
            <template #icon>
              <NIcon :component="ExportIcon" />
            </template>
            {{ t('settings.exportData') }}
          </NButton>
          <NButton @click="handleImport">
            <template #icon>
              <NIcon :component="ImportIcon" />
            </template>
            {{ t('settings.importData') }}
          </NButton>
        </NSpace>
        <NText depth="3" style="font-size: 12px; margin-top: 8px; display: block; line-height: 1.6">
          {{ t('settings.exportContent') }}
        </NText>
        <NText depth="3" style="font-size: 12px; display: block">
          {{ t('settings.fileFormat') }}
        </NText>
      </div>
    </div>

    <!-- 恢复预览弹窗 -->
    <NModal v-model:show="showRestoreModal" preset="card" :title="t('settings.restorePreview')" style="width: 400px">
      <div v-if="restorePreview" class="restore-preview-content">
        <!-- 备份基本信息 -->
        <div class="preview-section">
          <NText depth="2" style="font-weight: 500">{{ t('settings.backupInfo') }}</NText>
          <div class="preview-stats">
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.backupTime') }}</span>
              <span class="stat-value">{{ formatBackupTime(restorePreview.createdAt) }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.backupType') }}</span>
              <span class="stat-value">{{ formatBackupType(restorePreview.backupType) }}</span>
            </div>
          </div>
        </div>

        <!-- 数据统计 -->
        <div class="preview-section">
          <NText depth="2" style="font-weight: 500">{{ t('settings.dataStats') }}</NText>
          <div class="preview-stats">
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.categoriesCount') }}</span>
              <span class="stat-value">{{ restorePreview.categoriesCount }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.tasksCount') }}</span>
              <span class="stat-value">{{ restorePreview.tasksCount }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.clipboardItemsCount') }}</span>
              <span class="stat-value">{{ restorePreview.clipboardItemsCount }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">{{ t('settings.clipboardImageCount') }}</span>
              <span class="stat-value">{{ restorePreview.clipboardImageCount }}</span>
            </div>
          </div>
        </div>

        <!-- 恢复选项 -->
        <div class="preview-section">
          <NText depth="2" style="font-weight: 500">{{ t('settings.restoreOptions') }}</NText>

          <div class="option-row">
            <NSwitch
              :value="restoreOptions.overwrite"
              @update:value="(v: boolean) => { restoreOptions.overwrite = v; }"
            />
            <span class="option-label">{{ restoreOptions.overwrite ? t('settings.overwriteMode') : t('settings.mergeMode') }}</span>
          </div>

          <div class="option-row">
            <NCheckbox
              :checked="restoreOptions.restoreTasks"
              @update:checked="(v: boolean) => { restoreOptions.restoreTasks = v; }"
            />
            <span class="option-label">{{ t('settings.restoreTasks') }}</span>
          </div>

          <div class="option-row">
            <NCheckbox
              :checked="restoreOptions.restoreClipboard"
              @update:checked="(v: boolean) => { restoreOptions.restoreClipboard = v; }"
            />
            <span class="option-label">{{ t('settings.restoreClipboard') }}</span>
          </div>

          <div class="option-row">
            <NCheckbox
              :checked="restoreOptions.restoreSettings"
              @update:checked="(v: boolean) => { restoreOptions.restoreSettings = v; }"
            />
            <span class="option-label">{{ t('settings.restoreSettings') }}</span>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="restore-actions">
          <NButton type="primary" @click="doRestoreWithOptions">
            {{ t('settings.confirmRestore') }}
          </NButton>
          <NButton @click="showRestoreModal = false">
            {{ t('messages.cancel') }}
          </NButton>
        </div>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
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

  /* 设置项一行两个 */
  scrollbar-width: thin;
  scrollbar-color: rgba(100, 100, 100, 0.2) transparent;
}

/* 设置项一行布局 */
.settings-row {
  display: flex;
  gap: 24px;
  align-items: center;
}

.setting-item {
  display: flex;
  align-items: center;
  gap: 12px;
}

.setting-label {
  font-size: 14px;
  color: #333;
  min-width: 80px;
}

html.dark .setting-label {
  color: #e0e0e0;
}

.page-content {
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

.backup-section,
.data-section {
  margin-top: 8px;
}

.retention-value {
  margin-left: 12px;
  font-weight: 600;
  color: #4A90D9;
  font-size: 14px;
}

/* 备份列表 */
.backup-list {
  margin-top: 16px;
}

.backup-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 备份开关一行三个 */
.backup-switches {
  margin-top: 12px;
  gap: 16px;
}

.backup-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #fff;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
}

html.dark .backup-item {
  background: #2a2a2a;
  border-color: #444;
}

.backup-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.backup-time {
  font-size: 13px;
  color: #333;
}

html.dark .backup-time {
  color: #e0e0e0;
}

.backup-size {
  font-size: 12px;
  color: #888;
}

html.dark .backup-size {
  color: #999;
}

/* 备份类型标签 */
.backup-type-tag {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 500;
}

.backup-type-tag.quick {
  background: #e8f5e9;
  color: #28C840;
}

html.dark .backup-type-tag.quick {
  background: rgba(40, 200, 64, 0.15);
  color: rgba(40, 200, 64, 0.8);
}

.backup-type-tag.full {
  background: #fff3e0;
  color: #FFB800;
}

html.dark .backup-type-tag.full {
  background: rgba(255, 184, 0, 0.15);
  color: rgba(255, 184, 0, 0.8);
}

/* 备份按钮组 */
.backup-buttons {
  display: flex;
  gap: 12px;
  margin-top: 12px;
}

.backup-filename {
  font-size: 12px;
  color: #666;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

html.dark .backup-filename {
  color: #888;
}

.backup-actions {
  display: flex;
  gap: 4px;
}

.backup-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.backup-btn:hover {
  background: rgba(0, 0, 0, 0.08);
}

html.dark .backup-btn:hover {
  background: rgba(255, 255, 255, 0.1);
}

.backup-btn.restore:hover {
  color: #4A90D9;
}

.backup-btn.delete:hover {
  color: #E05252;
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

/* 层叠间距设置 */
.stack-gap-section {
  margin-top: 8px;
}

.gap-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.gap-value {
  min-width: 50px;
  font-weight: 600;
  color: #4A90D9;
  font-size: 14px;
  text-align: right;
}

/* 层叠预览卡片 */
.stack-preview {
  display: flex;
  flex-direction: column;
  gap: 0;
  padding: 16px;
  background: #f5f5f5;
  border-radius: 12px;
  margin-top: 8px;
}

html.dark .stack-preview {
  background: #1a1a1a;
}

.preview-card-stack {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  border-left: 3px solid #4A90D9;
  position: relative;
  transition: margin-bottom 0.2s ease;
}

html.dark .preview-card-stack {
  background: #2a2a2a;
  border-color: #444;
}

.preview-card-stack:last-child {
  margin-bottom: 0 !important;
}

.preview-check-stack {
  width: 16px;
  height: 16px;
  border: 2px solid #ccc;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 2px;
}

.preview-content-stack {
  flex: 1;
  min-width: 0;
}

.preview-title-stack {
  font-size: 14px;
  font-weight: 500;
  color: #333;
}

html.dark .preview-title-stack {
  color: #e0e0e0;
}

.preview-desc-stack {
  font-size: 12px;
  color: #888;
  margin-top: 4px;
}

html.dark .preview-desc-stack {
  color: #999;
}

/* 快捷键设置 */
.shortcut-section {
  margin-top: 8px;
}

.shortcut-input-wrapper {
  display: flex;
  align-items: center;
}

.shortcut-input {
  min-width: 180px;
  padding: 8px 12px;
  border: 1px solid #e0e0e0;
  border-radius: 6px;
  background: #fff;
  cursor: pointer;
  transition: all 0.15s;
  outline: none;
}

html.dark .shortcut-input {
  background: #2a2a2a;
  border-color: #444;
}

.shortcut-input:hover {
  border-color: #4A90D9;
}

.shortcut-input.active {
  border-color: #4A90D9;
  background: #e8f4fd;
}

html.dark .shortcut-input.active {
  background: rgba(74, 144, 217, 0.15);
}

.shortcut-input.error {
  border-color: #E05252;
}

.shortcut-display {
  font-size: 14px;
  font-weight: 500;
  color: #333;
}

html.dark .shortcut-display {
  color: #e0e0e0;
}

.placeholder {
  font-size: 14px;
  color: #999;
}

html.dark .placeholder {
  color: #666;
}

/* 恢复预览弹窗样式 */
.restore-preview-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.preview-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preview-stats {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.stat-label {
  font-size: 12px;
  color: #888;
}

html.dark .stat-label {
  color: #999;
}

.stat-value {
  font-size: 14px;
  font-weight: 500;
  color: #333;
}

html.dark .stat-value {
  color: #e0e0e0;
}

.option-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

.option-label {
  font-size: 14px;
  color: #333;
}

html.dark .option-label {
  color: #e0e0e0;
}

.restore-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 16px;
}

/* 快速恢复按钮样式 */
.backup-btn.quick-restore {
  background: #e8f5e9;
  color: #28C840;
}

html.dark .backup-btn.quick-restore {
  background: rgba(40, 200, 64, 0.15);
  color: rgba(40, 200, 64, 0.8);
}

.backup-btn.quick-restore:hover {
  background: #c8e6c9;
}

html.dark .backup-btn.quick-restore:hover {
  background: rgba(40, 200, 64, 0.25);
}
</style>