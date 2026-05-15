import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Task } from '../types';
import { getTasks, addTask, updateTask, deleteTask, reorderTasks, resetTaskSort } from '../utils/db';
import { FREE_TASK_PER_CATEGORY_LIMIT } from '../types';
import { useCategoryStore } from './categoryStore';

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([]);
  const searchQuery = ref('');
  const loading = ref(false);

  // 检查任务是否在锁定分类下
  function isTaskLocked(task: Task): boolean {
    const categoryStore = useCategoryStore();
    return categoryStore.isCategoryLocked(task.categoryId);
  }

  async function load() {
    loading.value = true;
    try {
      tasks.value = await getTasks();
    } finally {
      loading.value = false;
    }
  }

  async function add(taskData: Omit<Task, 'id' | 'createdAt' | 'updatedAt'>) {
    const count = tasks.value.filter(t => t.categoryId === taskData.categoryId).length;
    if (count >= FREE_TASK_PER_CATEGORY_LIMIT) throw new Error('该分类任务数量已达上限');
    const task = await addTask(taskData);
    tasks.value.push(task);
    return task;
  }

  // 快速添加空白任务（用于点击添加按钮直接创建）
  async function addQuickTask(categoryId: string) {
    const count = tasks.value.filter(t => t.categoryId === categoryId).length;
    if (count >= FREE_TASK_PER_CATEGORY_LIMIT) throw new Error('该分类任务数量已达上限');

    // 获取当前最小 sortOrder，新任务排在最前面
    const minSortOrder = tasks.value.length > 0
      ? Math.min(...tasks.value.map(t => t.sortOrder))
      : 0;
    const newSortOrder = minSortOrder - 1;

    const task = await addTask({
      categoryId,
      title: '待输入任务内容……',
      priority: 1,
      startDate: Date.now(),
      dueDate: undefined,
      status: 'todo',
      sortOrder: newSortOrder,
      isPinned: false,
      thumbnailBase64: undefined
    });

    tasks.value.unshift(task);
    return task;
  }

  async function update(task: Task) {
    task.updatedAt = Date.now();
    await updateTask(task);
    const index = tasks.value.findIndex(t => t.id === task.id);
    if (index !== -1) tasks.value[index] = task;
  }

  async function remove(id: string) {
    await deleteTask(id);
    tasks.value = tasks.value.filter(t => t.id !== id);
  }

  async function reorder(ids: string[]) {
    await reorderTasks(ids);
    await load();
  }

  async function resetSort() {
    await resetTaskSort();
    await load();
  }

  function toggleStatus(task: Task) {
    const newStatus = task.status === 'done' ? 'todo' : 'done';
    task.status = newStatus as 'todo' | 'done';
    // 完成时排到未完成任务尾部，取消完成时排到顶部
    if (newStatus === 'done') {
      const todoTasks = tasks.value.filter(t => t.status !== 'done');
      const maxSort = todoTasks.length > 0
        ? Math.max(...todoTasks.map(t => t.sortOrder))
        : (tasks.value.length > 0 ? Math.max(...tasks.value.filter(t => t.status === 'done').map(t => t.sortOrder)) : 0);
      task.sortOrder = maxSort + 1;
    } else {
      // 取消完成时排到未完成任务顶部
      const todoTasks = tasks.value.filter(t => t.status !== 'done');
      const minSort = todoTasks.length > 0
        ? Math.min(...todoTasks.map(t => t.sortOrder))
        : 0;
      task.sortOrder = minSort - 1;
    }
    update(task);
    // 重新排序以确保 UI 响应式更新
    tasks.value = [...tasks.value].sort((a, b) => a.sortOrder - b.sortOrder);
  }

  return {
    tasks,
    searchQuery,
    loading,
    load,
    add,
    addQuickTask,
    update,
    remove,
    reorder,
    resetSort,
    toggleStatus,
    isTaskLocked,
  };
});