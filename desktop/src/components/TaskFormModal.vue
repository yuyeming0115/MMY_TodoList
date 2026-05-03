<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  NModal, NForm, NFormItem, NInput,
  NDatePicker, NSelect, NSpace, NButton, NUpload
} from 'naive-ui';
import type { UploadFileInfo } from 'naive-ui';
import { useCategoryStore } from '../stores/categoryStore';
import { useTaskStore } from '../stores/taskStore';
import type { Task } from '../types';

const props = defineProps<{
  show: boolean;
  task?: Task | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'saved'): void;
}>();

const categoryStore = useCategoryStore();
const taskStore = useTaskStore();

const isEdit = computed(() => props.task != null);

// 表单数据
const form = ref({
  title: '',
  description: '',
  categoryId: '',
  startDate: undefined as number | undefined,
  dueDate: undefined as number | undefined,
  priority: 1 as number,
  thumbnailBase64: ''
});

// 分类选项
const categoryOptions = computed(() =>
  categoryStore.categories.map(c => ({
    label: c.name,
    value: c.id
  }))
);

// 优先级选项
const priorityOptions = [
  { label: '普通', value: 1 },
  { label: '重要', value: 2 },
  { label: '紧急', value: 3 }
];

// 图片上传处理
const fileList = ref<UploadFileInfo[]>([]);

function handleUpload({ file }: { file: UploadFileInfo }) {
  if (file.file) {
    const reader = new FileReader();
    reader.onload = (e) => {
      const base64 = e.target?.result as string;
      // 移除 data:image/xxx;base64, 前缀
      form.value.thumbnailBase64 = base64.split(',')[1] || '';
      // 更新 fileList 以显示预览
      fileList.value = [{
        id: file.id || 'new',
        name: file.name || 'thumbnail.jpg',
        status: 'finished',
        url: base64 // 完整的 data URL 用于预览
      }];
    };
    reader.readAsDataURL(file.file);
  }
  return false; // 阻止自动上传
}

function removeImage() {
  form.value.thumbnailBase64 = '';
  fileList.value = [];
}

// 初始化表单
watch(() => props.show, (show) => {
  if (show) {
    if (props.task) {
      // 编辑模式
      form.value = {
        title: props.task.title,
        description: props.task.description || '',
        categoryId: props.task.categoryId,
        startDate: props.task.startDate,
        dueDate: props.task.dueDate,
        priority: props.task.priority,
        thumbnailBase64: props.task.thumbnailBase64 || ''
      };
      if (props.task.thumbnailBase64) {
        fileList.value = [{
          id: 'existing',
          name: 'thumbnail.jpg',
          status: 'finished',
          url: `data:image/jpeg;base64,${props.task.thumbnailBase64}`
        }];
      } else {
        fileList.value = [];
      }
    } else {
      // 新建模式
      form.value = {
        title: '',
        description: '',
        categoryId: categoryStore.categories[0]?.id || '',
        startDate: Date.now(),
        dueDate: undefined,
        priority: 1,
        thumbnailBase64: ''
      };
      fileList.value = [];
    }
  }
});

// 保存
async function handleSave() {
  if (!form.value.title.trim()) return;
  if (!form.value.categoryId) return;

  try {
    if (isEdit.value && props.task) {
      // 更新
      const updated: Task = {
        ...props.task,
        title: form.value.title.trim(),
        description: form.value.description.trim() || undefined,
        categoryId: form.value.categoryId,
        startDate: form.value.startDate,
        dueDate: form.value.dueDate,
        priority: form.value.priority as 1 | 2 | 3,
        thumbnailBase64: form.value.thumbnailBase64 || undefined,
        updatedAt: Date.now()
      };
      await taskStore.update(updated);
    } else {
      // 新建
      await taskStore.add({
        categoryId: form.value.categoryId,
        title: form.value.title.trim(),
        description: form.value.description.trim() || undefined,
        startDate: form.value.startDate,
        dueDate: form.value.dueDate,
        priority: form.value.priority as 1 | 2 | 3,
        status: 'todo',
        sortOrder: 0,
        isPinned: false,
        thumbnailBase64: form.value.thumbnailBase64 || undefined
      });
    }
    emit('saved');
    emit('close');
  } catch (e) {
    console.error(e);
  }
}

function handleClose() {
  emit('close');
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    :title="isEdit ? '编辑任务' : '添加任务'"
    style="width: 360px"
    :mask-closable="false"
    @close="handleClose"
  >
    <NForm label-placement="left" label-width="80">
      <NFormItem label="标题" required>
        <NInput
          v-model:value="form.title"
          placeholder="输入任务标题"
          maxlength="100"
        />
      </NFormItem>

      <NFormItem label="描述">
        <NInput
          v-model:value="form.description"
          type="textarea"
          placeholder="任务描述（可选）"
          maxlength="500"
          :rows="3"
        />
      </NFormItem>

      <NFormItem label="分类">
        <NSelect
          v-model:value="form.categoryId"
          :options="categoryOptions"
          placeholder="选择分类"
        />
      </NFormItem>

      <NFormItem label="开始日期">
        <NDatePicker
          :value="form.startDate ?? null"
          @update:value="(v: number | null) => form.startDate = v ?? undefined"
          type="date"
          clearable
          style="width: 100%"
        />
      </NFormItem>

      <NFormItem label="截止日期">
        <NDatePicker
          :value="form.dueDate ?? null"
          @update:value="(v: number | null) => form.dueDate = v ?? undefined"
          type="date"
          clearable
          style="width: 100%"
        />
      </NFormItem>

      <NFormItem label="优先级">
        <NSelect
          v-model:value="form.priority"
          :options="priorityOptions"
        />
      </NFormItem>

      <NFormItem label="参考图">
        <NSpace vertical>
          <NUpload
            :file-list="fileList"
            :max="1"
            accept="image/*"
            list-type="image-card"
            @change="handleUpload"
            @remove="removeImage"
          >
            <NButton size="small">上传图片</NButton>
          </NUpload>
          <span style="color: #888; font-size: 12px">每个任务限 1 张图片</span>
        </NSpace>
      </NFormItem>
    </NForm>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="handleClose">取消</NButton>
        <NButton type="primary" @click="handleSave" :disabled="!form.title.trim()">
          保存
        </NButton>
      </NSpace>
    </template>
  </NModal>
</template>