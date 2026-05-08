<script setup lang="ts">
import { NIcon, NDropdown, NInput, useMessage } from 'naive-ui';
import { h, ref, computed, nextTick } from 'vue';
import {
  TrashOutline as DeleteIcon, CopyOutline as CopyIcon, StarOutline as StarIcon, Star as StarFilledIcon,
  CreateOutline as EditIcon
} from '@vicons/ionicons5';
import type { ClipboardItem } from '../types';
import { useClipboardStore } from '../stores/clipboardStore';
import { BUILTIN_CLIPBOARD_CATEGORIES } from '../types';

const props = defineProps<{
  item: ClipboardItem;
  compact?: boolean;
}>();

const emit = defineEmits<{
  (e: 'delete', id: string): void;
  (e: 'update-priority', item: ClipboardItem, priority: 1 | 2 | 3): void;
  (e: 'contextmenu', event: MouseEvent, item: ClipboardItem): void;
}>();

const message = useMessage();
const clipboardStore = useClipboardStore();
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

const isFavorite = computed(() => props.item.categoryId === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE);
const isTextItem = computed(() => !props.item.imageBase64);

const displayContent = computed(() => {
  if (!props.compact) return props.item.content;
  return props.item.content.length > 50
    ? props.item.content.substring(0, 50) + '…'
    : props.item.content;
});

// 编辑模式
const isEditing = ref(false);
const editTitle = ref('');
const editContent = ref('');
const editTextareaRef = ref<any>(null);

const contextMenuOptions = computed(() => {
  const options: any[] = [
    { label: '复制', key: 'copy', icon: () => h(NIcon, { component: CopyIcon, size: 16 }) },
    { label: isFavorite.value ? '取消收藏' : '收藏', key: 'favorite', icon: () => h(NIcon, { component: isFavorite.value ? StarFilledIcon : StarIcon, size: 16, style: { color: isFavorite.value ? '#F39C12' : '#333' } }) },
  ];

  // 只有文本类型才显示编辑
  if (isTextItem.value) {
    options.push({ label: '编辑', key: 'edit', icon: () => h(NIcon, { component: EditIcon, size: 16 }) });
  }

  options.push({ type: 'divider', key: 'd1' });
  options.push({ label: '删除', key: 'delete', icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } }) });
  return options;
});

async function copyContent() {
  try {
    if (props.item.imageBase64) {
      const { invoke } = await import('@tauri-apps/api/core');
      const base64Data = props.item.imageBase64.replace(/^data:image\/\w+;base64,/, '');
      await invoke('write_image_to_clipboard', { base64: base64Data });
      message.success('已复制图片');
    } else if (props.item.content) {
      await navigator.clipboard.writeText(props.item.content);
      message.success('已复制');
    }
  } catch (e) {
    if (props.item.content) {
      const ta = document.createElement('textarea');
      ta.value = props.item.content;
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      if (ok) {
        message.success('已复制');
        return;
      }
    }
    message.error('复制失败');
    console.error('复制失败:', e);
  }
}

async function handleFavorite() {
  const result = await clipboardStore.favoriteItem(props.item);
  if (result === 'favorited') {
    message.success('已收藏');
  } else if (result === 'unfavorited') {
    message.success('已取消收藏');
  } else {
    message.error('收藏分类不存在，请刷新后重试');
  }
}

function handleClick() {
  copyContent();
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  showContextMenu.value = true;
}

function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'copy') copyContent();
  if (key === 'favorite') handleFavorite();
  if (key === 'edit') startEdit();
  if (key === 'delete') emit('delete', props.item.id);
}

async function startEdit() {
  isEditing.value = true;
  editTitle.value = props.item.title;
  editContent.value = props.item.content;
  await nextTick();
  editTextareaRef.value?.focus();
}

function saveEdit() {
  if (!editTitle.value.trim() && !editContent.value.trim()) {
    // 如果都为空，取消编辑
    cancelEdit();
    return;
  }
  const updated: ClipboardItem = {
    ...props.item,
    title: editTitle.value.trim() || props.item.title,
    content: editContent.value,
  };
  clipboardStore.updateItem(updated);
  isEditing.value = false;
  message.success('已保存');
}

function cancelEdit() {
  isEditing.value = false;
  editTitle.value = '';
  editContent.value = '';
}
</script>

<template>
  <NDropdown
    placement="bottom-start"
    trigger="manual"
    :x="contextMenuX"
    :y="contextMenuY"
    :show="showContextMenu"
    :options="contextMenuOptions"
    @select="handleMenuSelect"
    @clickoutside="showContextMenu = false"
  />
  <div class="task-card" :class="{ compact: props.compact }" @click="handleClick" @contextmenu="handleContextMenu">
    <div class="task-content">
      <!-- 纯图片：只显示图片 -->
      <div v-if="item.imageBase64" class="image-only">
        <div class="task-thumbnail" :class="{ 'compact-thumb': props.compact }">
          <img :src="item.imageBase64" alt="剪贴板图片" />
        </div>
      </div>

      <!-- 文本：标题 + 内容 -->
      <div v-if="!item.imageBase64">
        <!-- 编辑模式 -->
        <div v-if="isEditing" class="edit-mode" @click.stop>
          <NInput
            v-model:value="editTitle"
            size="small"
            placeholder="标题"
            class="edit-title-input"
            @keyup.enter="saveEdit"
          />
          <NInput
            ref="editTextareaRef"
            v-model:value="editContent"
            type="textarea"
            size="small"
            placeholder="内容"
            :autosize="{ minRows: 2, maxRows: 6 }"
            class="edit-content-input"
          />
          <div class="edit-actions">
            <button class="edit-btn save" @click="saveEdit">保存</button>
            <button class="edit-btn cancel" @click="cancelEdit">取消</button>
          </div>
        </div>
        <!-- 正常显示 -->
        <template v-else>
          <div v-if="item.content" class="task-desc">{{ displayContent }}</div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-card {
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
  transition: box-shadow 0.2s, border-color 0.2s, transform 0.2s;
  cursor: pointer;
  transform-origin: center center;
}

.task-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  border-color: #4A90D9;
  transform: scale(1.01);
}

html.dark .task-card {
  background: #2a2a2a;
  border-color: #444;
}

html.dark .task-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  border-color: #4A90D9;
  transform: scale(1.01);
}

.task-card:active {
  transform: scale(0.99);
}

/* 精简模式样式 */
.task-card.compact {
  padding: 8px 10px;
  border-radius: 10px;
  border-left-width: 3px;
}

/* 精简模式禁用 hover 放大，避免被 overflow 裁切 */
.task-card.compact:hover {
  transform: none;
}

.task-card.compact .task-title {
  font-size: 15px;
  font-weight: 600;
  color: #1a1a1a;
}

html.dark .task-card.compact .task-title {
  color: #f0f0f0;
}

.task-card.compact .task-desc {
  font-size: 12px;
  line-height: 1.4;
  max-height: 36px;
  -webkit-line-clamp: 2;
}

.task-card.compact .task-thumbnail.compact-thumb {
  max-width: 120px;
}

.task-card.compact .task-thumbnail.compact-thumb img {
  max-height: 80px;
  object-fit: cover;
}

.task-content {
  min-width: 0;
}

.task-title {
  font-size: var(--task-font-size, 14px);
  font-weight: 500;
  color: #333;
  word-break: break-all;
}

.task-card:hover .task-title {
  color: #FFB800;
}

html.dark .task-card:hover .task-title {
  color: #FFB800;
}

html.dark .task-title {
  color: #e0e0e0;
}

.task-desc {
  font-size: calc(var(--task-font-size, 14px) * 0.857);
  color: #888;
  margin-top: 4px;
  word-break: break-all;
  white-space: pre-wrap;
  max-height: 60px;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  transition: color 0.15s;
}

.task-card:hover .task-desc {
  color: #FFB800;
}

html.dark .task-desc {
  color: #999;
}

html.dark .task-card:hover .task-desc {
  color: #FFB800;
}

.image-only {
  width: 100%;
}

.task-thumbnail {
  max-width: 200px;
  border-radius: 8px;
  overflow: hidden;
}

.task-thumbnail img {
  width: 100%;
  display: block;
}

/* 编辑模式 */
.edit-mode {
  width: 100%;
}

.edit-title-input {
  margin-bottom: 8px;
}

.edit-content-input {
  margin-bottom: 8px;
}

.edit-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.edit-btn {
  padding: 4px 16px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.edit-btn.save {
  background: #4A90D9;
  color: #fff;
}

.edit-btn.save:hover {
  background: #3A7BC8;
}

.edit-btn.cancel {
  background: transparent;
  color: #999;
  border: 1px solid #ddd;
}

.edit-btn.cancel:hover {
  background: rgba(0, 0, 0, 0.05);
}

html.dark .edit-btn.cancel {
  border-color: #444;
  color: #888;
}

html.dark .edit-btn.cancel:hover {
  background: rgba(255, 255, 255, 0.05);
}
</style>
