<script setup lang="ts">
import { ref, computed, h, watch, nextTick } from 'vue';
import { NIcon, NCheckbox, NDropdown, NDatePicker, NPopover } from 'naive-ui';
import {
  StarOutline as StarOutlineIcon,
  Star as StarIcon,
  ChevronDownOutline as ExpandIcon,
  CreateOutline as EditIcon,
  TrashOutline as DeleteIcon,
  FolderOutline as FolderIcon,
  CalendarOutline as CalendarIcon,
  TimeOutline as ClockIcon,
  PinOutline as PinOutlineIcon,
  Pin as PinIcon
} from '@vicons/ionicons5';
import type { Task, Category } from '../types';

const props = defineProps<{
  task: Task;
  categoryColor?: string;
  categories: Category[];
  isEditingTitle?: boolean;
}>();
const emit = defineEmits<{
  (e: 'edit', task: Task): void;
  (e: 'delete', id: string): void;
  (e: 'toggleStatus', task: Task): void;
  (e: 'togglePin', task: Task): void;
  (e: 'updatePriority', task: Task, priority: 1 | 2 | 3): void;
  (e: 'updateCategory', task: Task, categoryId: string): void;
  (e: 'updateStartDate', task: Task, date: number | undefined): void;
  (e: 'updateDueDate', task: Task, date: number | undefined): void;
  (e: 'updateTitle', task: Task, title: string): void;
  (e: 'updateDescription', task: Task, description: string | undefined): void;
}>();

const isDone = computed(() => props.task.status === 'done');

// 标题编辑状态
const isEditing = ref(false);
const editTitleValue = ref('');
const titleInputRef = ref<HTMLInputElement | null>(null);

// 监听外部编辑状态（新添加的任务自动进入编辑）
watch(() => props.isEditingTitle, (val) => {
  if (val && !isEditing.value) {
    startEditTitle();
  }
});

// 开始编辑标题
function startEditTitle() {
  isEditing.value = true;
  // 如果是默认文字，清空；否则保留原标题
  editTitleValue.value = props.task.title === '待输入任务内容……' ? '' : props.task.title;
  nextTick(() => {
    titleInputRef.value?.focus();
  });
}

// 保存标题
function saveTitle() {
  const newTitle = editTitleValue.value.trim() || '待输入任务内容……';
  emit('updateTitle', props.task, newTitle);
  isEditing.value = false;
}

// 取消编辑标题
function cancelEdit() {
  isEditing.value = false;
  editTitleValue.value = '';
}

// 描述编辑状态
const isEditingDesc = ref(false);
const editDescValue = ref('');
const descInputRef = ref<HTMLInputElement | null>(null);

// 开始编辑描述
function startEditDesc() {
  isEditingDesc.value = true;
  editDescValue.value = props.task.description || '';
  nextTick(() => {
    descInputRef.value?.focus();
  });
}

// 保存描述
function saveDesc() {
  const newDesc = editDescValue.value.trim() || undefined;
  emit('updateDescription', props.task, newDesc);
  isEditingDesc.value = false;
}

// 取消编辑描述
function cancelDescEdit() {
  isEditingDesc.value = false;
  editDescValue.value = '';
}

// 优先级星级 (1=高=3星, 2=中=2星, 3=低=1星)
const priorityStars = computed(() => props.task.priority);

// 点击星星设置优先级
function handleSetPriority(starIndex: number) {
  const newPriority = starIndex as 1 | 2 | 3;
  emit('updatePriority', props.task, newPriority);
}

// 倒计天数计算
const countdownDays = computed(() => {
  if (!props.task.dueDate) return null;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dueDate = new Date(props.task.dueDate);
  dueDate.setHours(0, 0, 0, 0);
  const diffDays = Math.ceil((dueDate.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));
  return diffDays;
});

// 倒计天数显示文本和样式
const countdownText = computed(() => {
  if (countdownDays.value === null) return '';
  if (countdownDays.value < 0) return '已过期';
  if (countdownDays.value === 0) return '今天';
  if (countdownDays.value === 1) return '明天';
  return `${countdownDays.value}天`;
});

const countdownClass = computed(() => {
  if (countdownDays.value === null) return '';
  if (countdownDays.value < 0) return 'overdue';
  if (countdownDays.value <= 1) return 'urgent';
  if (countdownDays.value <= 3) return 'soon';
  return 'normal';
});

// 格式化日期显示
function formatDate(timestamp: number | undefined): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  const month = date.getMonth() + 1;
  const day = date.getDate();
  return `${month}/${day}`;
}

// 更新开始日期
function handleStartDateChange(value: number | null) {
  emit('updateStartDate', props.task, value ?? undefined);
}

// 更新截止日期
function handleDueDateChange(value: number | null) {
  emit('updateDueDate', props.task, value ?? undefined);
}

// 右键菜单选项
const contextMenuOptions = computed(() => {
  const categoryOptions = props.categories.map(cat => ({
    label: cat.name,
    key: `cat-${cat.id}`,
    icon: () => h(NIcon, {
      component: FolderIcon,
      size: 16,
      style: { color: cat.color }
    })
  }));

  return [
    {
      label: '编辑',
      key: 'edit',
      icon: () => h(NIcon, { component: EditIcon, size: 16 })
    },
    {
      label: '删除',
      key: 'delete',
      icon: () => h(NIcon, { component: DeleteIcon, size: 16, style: { color: '#E05252' } })
    },
    { type: 'divider', key: 'd1' },
    ...categoryOptions
  ];
});

// 右键菜单显示状态
const showContextMenu = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

// 右键点击显示菜单
function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  showContextMenu.value = true;
}

// 处理菜单选择
function handleMenuSelect(key: string) {
  showContextMenu.value = false;
  if (key === 'edit') {
    emit('edit', props.task);
  } else if (key === 'delete') {
    emit('delete', props.task.id);
  } else if (key.startsWith('cat-')) {
    const categoryId = key.slice(4);
    if (categoryId !== props.task.categoryId) {
      emit('updateCategory', props.task, categoryId);
    }
  }
}

// 展开/折叠状态
const isExpanded = ref(false);

// 是否需要展开功能（描述长度判断）
const needsExpand = computed(() =>
  props.task.description && props.task.description.length > 30
);

// 点击展开/折叠
function toggleExpand(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 排除交互元素
  if (target.closest('.n-checkbox, .star-icon, .expand-icon, .date-picker-trigger, .n-date-picker, .task-title, .title-input, .right-area')) return;
  if (needsExpand.value) {
    isExpanded.value = !isExpanded.value;
  }
}

function handleToggleStatus() {
  emit('toggleStatus', props.task);
}
</script>

<template>
  <!-- 右键菜单 -->
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
  <div
    class="simple-card"
    :class="{ done: isDone, expanded: isExpanded, pinned: props.task.isPinned }"
    :style="{ borderLeftColor: categoryColor || 'transparent', borderLeftWidth: categoryColor ? '3px' : '0' }"
    @click="toggleExpand"
    @contextmenu="handleContextMenu"
  >
    <!-- 置顶按钮 -->
    <span class="pin-btn" @click.stop="emit('togglePin', props.task)">
      <NIcon :component="props.task.isPinned ? PinIcon : PinOutlineIcon" size="16" />
    </span>

    <!-- 完成状态复选框 -->
    <NCheckbox
      :checked="isDone"
      size="small"
      @update:checked="handleToggleStatus"
      class="status-checkbox"
    />

    <!-- 缩略图（如果有） -->
    <div v-if="props.task.thumbnailBase64" class="thumbnail-wrapper">
      <img
        :src="`data:image/jpeg;base64,${props.task.thumbnailBase64}`"
        class="thumbnail"
        alt=""
      />
    </div>

    <!-- 任务内容 -->
    <div class="task-content">
      <div class="task-header">
        <!-- 标题（可编辑） -->
        <span v-if="!isEditing" class="task-title" @click="startEditTitle">
          {{ props.task.title }}
        </span>
        <input
          v-else
          ref="titleInputRef"
          v-model="editTitleValue"
          class="title-input"
          placeholder="输入任务内容"
          @blur="saveTitle"
          @keyup.enter="saveTitle"
          @keyup.escape="cancelEdit"
        />
        <!-- 右侧区域：星标 + 时间 -->
        <span class="right-area">
          <!-- 星标优先级（可点击设置） -->
          <span class="priority-stars">
            <NIcon
              v-for="i in 3"
              :key="i"
              :component="i <= priorityStars ? StarIcon : StarOutlineIcon"
              size="14"
              :class="{ star: i <= priorityStars }"
              class="star-icon"
              @click.stop="handleSetPriority(i)"
            />
          </span>
          <!-- 时间区域 -->
          <span class="time-area">
            <!-- 开始时间 -->
            <NPopover trigger="click" placement="bottom" :show-arrow="false">
              <template #trigger>
                <span class="date-display">
                  <NIcon :component="CalendarIcon" size="12" />
                  <span>{{ formatDate(props.task.startDate) || '开始' }}</span>
                </span>
              </template>
              <NDatePicker
                :value="props.task.startDate ?? null"
                type="date"
                size="small"
                :default-value="Date.now()"
                @update:value="handleStartDateChange"
                style="width: 200px"
              />
            </NPopover>

            <!-- 截止时间 -->
            <NPopover trigger="click" placement="bottom" :show-arrow="false">
              <template #trigger>
                <span class="date-display">
                  <NIcon :component="ClockIcon" size="12" />
                  <span>{{ formatDate(props.task.dueDate) || '截止' }}</span>
                </span>
              </template>
              <NDatePicker
                :value="props.task.dueDate ?? null"
                type="date"
                size="small"
                :default-value="props.task.startDate ?? Date.now()"
                @update:value="handleDueDateChange"
                style="width: 200px"
              />
            </NPopover>

            <!-- 倒计天数 -->
            <span v-if="countdownText" class="countdown" :class="countdownClass">
              {{ countdownText }}
            </span>
          </span>
        </span>
        <!-- 展开/折叠提示图标 -->
        <span v-if="needsExpand" class="expand-icon" :class="{ expanded: isExpanded }">
          <NIcon :component="ExpandIcon" size="14" />
        </span>
      </div>
      <!-- 描述（可点击编辑） -->
      <div
        v-if="props.task.description || isEditingDesc"
        class="task-desc"
        :class="{ expanded: isExpanded }"
        @click="startEditDesc"
      >
        <span v-if="!isEditingDesc">{{ props.task.description }}</span>
        <input
          v-else
          ref="descInputRef"
          v-model="editDescValue"
          class="desc-input"
          placeholder="添加描述..."
          @blur="saveDesc"
          @keyup.enter="saveDesc"
          @keyup.escape="cancelDescEdit"
          @click.stop
        />
      </div>
      <!-- 添加描述提示（没有描述时显示） -->
      <div v-else class="task-desc-empty" @click="startEditDesc">
        点击添加描述...
      </div>
    </div>
  </div>
</template>

<style scoped>
.simple-card {
  width: 100%;
  padding: 12px 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
  transition: box-shadow 0.2s ease, border-color 0.2s ease;
  user-select: none;
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

html.dark .simple-card {
  background: #2a2a2a;
  border-color: #444;
}

.simple-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

html.dark .simple-card:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.simple-card.done {
  opacity: 0.5;
}

.simple-card.pinned {
  background: linear-gradient(135deg, #fff 0%, #f8f4e8 100%);
}

html.dark .simple-card.pinned {
  background: linear-gradient(135deg, #2a2a2a 0%, #353025 100%);
}

.pin-btn {
  display: flex;
  align-items: center;
  color: #ccc;
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s ease, transform 0.15s ease;
  margin-right: 4px;
}

.pin-btn:hover {
  color: #FFB800;
  transform: scale(1.1);
}

.simple-card.pinned .pin-btn {
  color: #FFB800;
}

.simple-card.done {
  opacity: 0.5;
}

.status-checkbox {
  flex-shrink: 0;
  margin-top: 2px;
}

.thumbnail-wrapper {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  overflow: hidden;
  background: #f0f0f0;
}

html.dark .thumbnail-wrapper {
  background: #333;
}

.thumbnail {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.task-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.task-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.task-title {
  font-size: 14px;
  color: #333;
  font-weight: 500;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

html.dark .task-title {
  color: #e0e0e0;
}

.simple-card.done .task-title {
  text-decoration: line-through;
}

.title-input {
  font-size: 14px;
  font-weight: 500;
  color: #333;
  background: transparent;
  border: none;
  outline: none;
  flex: 1;
  min-width: 100px;
  padding: 0;
  font-family: inherit;
}

html.dark .title-input {
  color: #e0e0e0;
}

.title-input::placeholder {
  color: #999;
}

html.dark .title-input::placeholder {
  color: #666;
}

.right-area {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  flex-shrink: 0;
}

.priority-stars {
  display: flex;
  gap: 2px;
  color: #ccc;
}

.priority-stars .star-icon {
  cursor: pointer;
  transition: color 0.15s ease, transform 0.15s ease;
}

.priority-stars .star-icon:hover {
  transform: scale(1.15);
}

.priority-stars .star {
  color: #FFB800;
}

.time-area {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}

.date-display {
  display: flex;
  align-items: center;
  gap: 3px;
  color: #888;
  cursor: pointer;
  padding: 1px 4px;
  border-radius: 4px;
  transition: background 0.15s ease;
}

html.dark .date-display {
  color: #999;
}

.date-display:hover {
  background: rgba(74, 144, 217, 0.1);
  color: #4A90D9;
}

html.dark .date-display:hover {
  background: rgba(74, 144, 217, 0.15);
}

.countdown {
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 11px;
  font-weight: 500;
}

.countdown.normal {
  background: #e8f5e9;
  color: #28C840;
}

.countdown.soon {
  background: #fff3e0;
  color: #FF9500;
}

.countdown.urgent {
  background: #ffebee;
  color: #E05252;
}

.countdown.overdue {
  background: #f5f5f5;
  color: #999;
}

html.dark .countdown.normal {
  background: rgba(40, 200, 64, 0.15);
  color: #28C840;
}

html.dark .countdown.soon {
  background: rgba(255, 149, 0, 0.15);
  color: #FF9500;
}

html.dark .countdown.urgent {
  background: rgba(224, 82, 82, 0.15);
  color: #E05252;
}

html.dark .countdown.overdue {
  background: rgba(100, 100, 100, 0.15);
  color: #888;
}

.expand-icon {
  color: #4A90D9;
  flex-shrink: 0;
  transition: transform 0.3s ease;
  cursor: pointer;
  margin-top: 2px;
}

.expand-icon.expanded {
  transform: rotate(180deg);
}

.task-desc {
  font-size: 12px;
  color: #888;
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-height: 18px;
  transition: max-height 0.3s ease, white-space 0.3s ease;
}

html.dark .task-desc {
  color: #999;
}

.task-desc.expanded {
  white-space: normal;
  max-height: 60px;
  overflow-y: auto;
}

.task-desc-empty {
  font-size: 12px;
  color: #999;
  margin-top: 4px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
  transition: background 0.15s ease;
}

html.dark .task-desc-empty {
  color: #666;
}

.task-desc-empty:hover {
  background: rgba(74, 144, 217, 0.1);
  color: #4A90D9;
}

html.dark .task-desc-empty:hover {
  background: rgba(74, 144, 217, 0.15);
}

.desc-input {
  font-size: 12px;
  color: #888;
  background: transparent;
  border: none;
  outline: none;
  width: 100%;
  padding: 0;
  font-family: inherit;
}

html.dark .desc-input {
  color: #999;
}

.desc-input::placeholder {
  color: #aaa;
}
</style>