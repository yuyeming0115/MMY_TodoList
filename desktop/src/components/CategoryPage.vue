<script setup lang="ts">
import { ref } from 'vue';
import {
  NButton, NSpace, NInput, NColorPicker, NIcon, NEmpty, NText,
  NList, NListItem, useDialog
} from 'naive-ui';
import {
  AddOutline as AddIcon, CreateOutline as EditIcon,
  TrashOutline as DeleteIcon, ArrowBackOutline as BackIcon
} from '@vicons/ionicons5';
import { useCategoryStore } from '../stores/categoryStore';
import { useMessage } from 'naive-ui';
import { FREE_CATEGORY_LIMIT } from '../types';

const emit = defineEmits<{
  (e: 'back'): void;
}>();

const categoryStore = useCategoryStore();
const message = useMessage();
const dialog = useDialog();

// 新建/编辑状态
const isAdding = ref(false);
const isEditing = ref(false);
const editingId = ref('');
const formName = ref('');
const formColor = ref('#4A90D9');

// 预设颜色
const presetColors = [
  '#4A90D9', '#E05252', '#FF9500', '#28C840',
  '#9B59B6', '#3498DB', '#1ABC9C', '#F39C12'
];

// 开始新建
function startAdd() {
  if (!categoryStore.canAddMore) {
    message.warning(`分类数量已达上限（${FREE_CATEGORY_LIMIT}个）`);
    return;
  }
  isAdding.value = true;
  isEditing.value = false;
  formName.value = '';
  formColor.value = '#4A90D9';
}

// 开始编辑
function startEdit(id: string, name: string, color: string) {
  isAdding.value = false;
  isEditing.value = true;
  editingId.value = id;
  formName.value = name;
  formColor.value = color;
}

// 保存
async function handleSave() {
  if (!formName.value.trim()) {
    message.warning('请输入分类名称');
    return;
  }

  try {
    if (isAdding.value) {
      await categoryStore.add(formName.value.trim(), formColor.value);
      message.success('添加成功');
    } else if (isEditing.value) {
      const cat = categoryStore.categories.find(c => c.id === editingId.value);
      if (cat) {
        cat.name = formName.value.trim();
        cat.color = formColor.value;
        await categoryStore.update(cat);
        message.success('更新成功');
      }
    }
    cancelForm();
  } catch (e) {
    message.error('操作失败');
  }
}

// 删除
function handleDelete(id: string, name: string) {
  dialog.warning({
    title: '确认删除',
    content: `确定删除分类"${name}"及其所有任务？`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await categoryStore.remove(id);
      message.success('删除成功');
    }
  });
}

// 取消表单
function cancelForm() {
  isAdding.value = false;
  isEditing.value = false;
  editingId.value = '';
  formName.value = '';
  formColor.value = '#4A90D9';
}

function goBack() {
  cancelForm();
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
      <span class="page-title">分类管理</span>
    </header>

    <!-- 页面内容 -->
    <div class="page-content">
      <!-- 分类列表 -->
      <div v-if="!isAdding && !isEditing">
        <NList bordered>
          <NListItem v-for="cat in categoryStore.categories" :key="cat.id">
            <template #prefix>
              <span
                class="color-dot"
                :style="{ backgroundColor: cat.color }"
              />
            </template>
            <NText>{{ cat.name }}</NText>
            <template #suffix>
              <NSpace :size="8">
                <NButton text size="tiny" @click="startEdit(cat.id, cat.name, cat.color)">
                  <template #icon>
                    <NIcon :component="EditIcon" />
                  </template>
                </NButton>
                <NButton text size="tiny" @click="handleDelete(cat.id, cat.name)">
                  <template #icon>
                    <NIcon :component="DeleteIcon" color="#E05252" />
                  </template>
                </NButton>
              </NSpace>
            </template>
          </NListItem>
        </NList>

        <NEmpty v-if="categoryStore.categories.length === 0" description="暂无分类" style="margin: 20px 0" />
      </div>

      <!-- 新建/编辑表单 -->
      <div v-else class="form-area">
        <NSpace vertical :size="16">
          <NInput
            v-model:value="formName"
            placeholder="分类名称"
            maxlength="20"
            size="large"
          />
          <div>
            <NText depth="3" style="margin-bottom: 8px; display: block">选择颜色</NText>
            <NColorPicker
              v-model:value="formColor"
              :swatches="presetColors"
              :modes="['hex']"
            />
          </div>
        </NSpace>
      </div>
    </div>

    <!-- 页面底部 -->
    <footer class="page-footer">
      <div v-if="!isAdding && !isEditing">
        <NButton type="primary" block size="large" @click="startAdd" :disabled="!categoryStore.canAddMore">
          <template #icon>
            <NIcon :component="AddIcon" />
          </template>
          添加分类
        </NButton>
        <NText v-if="!categoryStore.canAddMore" depth="3" style="margin-top: 8px; display: block; text-align: center; font-size: 12px">
          已达上限 {{ FREE_CATEGORY_LIMIT }} 个
        </NText>
      </div>
      <NSpace v-else justify="end" :size="12" style="width: 100%">
        <NButton size="large" @click="cancelForm">取消</NButton>
        <NButton type="primary" size="large" @click="handleSave">保存</NButton>
      </NSpace>
    </footer>
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

.page-footer {
  padding: 12px 16px;
  border-top: 1px solid #e0e0e0;
  background: #fafafa;
  flex-shrink: 0;
}

html.dark .page-footer {
  background: #242424;
  border-top-color: #333;
}

.color-dot {
  display: inline-block;
  width: 16px;
  height: 16px;
  border-radius: 4px;
}

.form-area {
  padding: 8px 0;
}
</style>