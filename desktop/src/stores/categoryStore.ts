import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { Category } from '../types';
import { getCategories, addCategory, updateCategory, deleteCategory, ensureDefaultCategory as invokeEnsureDefault, reorderCategories as invokeReorderCategories } from '../utils/db';
import { FREE_CATEGORY_LIMIT } from '../types';

export const useCategoryStore = defineStore('category', () => {
  const categories = ref<Category[]>([]);
  const selectedCategoryId = ref<string | null>(null); // null = 全部
  const loading = ref(false);

  const canAddMore = computed(() => categories.value.length < FREE_CATEGORY_LIMIT);

  async function load() {
    loading.value = true;
    try {
      categories.value = await getCategories();
    } finally {
      loading.value = false;
    }
  }

  async function add(name: string, color: string) {
    if (!canAddMore.value) throw new Error('分类数量已达上限');
    const category = await addCategory(name, color);
    categories.value.push(category);
    return category;
  }

  async function update(category: Category) {
    await updateCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  // 锁定/解锁分类
  async function lockCategory(category: Category): Promise<void> {
    category.locked = true;
    await updateCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  async function unlockCategory(category: Category): Promise<void> {
    category.locked = false;
    await updateCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  async function toggleCategoryLock(category: Category): Promise<'locked' | 'unlocked'> {
    if (category.locked) {
      await unlockCategory(category);
      return 'unlocked';
    } else {
      await lockCategory(category);
      return 'locked';
    }
  }

  // 检查任务是否在锁定分类下
  function isCategoryLocked(categoryId: string): boolean {
    const category = categories.value.find(c => c.id === categoryId);
    return category?.locked === true;
  }

  async function remove(id: string) {
    await deleteCategory(id);
    categories.value = categories.value.filter(c => c.id !== id);
    if (selectedCategoryId.value === id) selectedCategoryId.value = null;
  }

  function select(id: string | null) {
    selectedCategoryId.value = id;
  }

  async function ensureDefaultCategory() {
    if (categories.value.length > 0) return categories.value[0].id;
    try {
      await invokeEnsureDefault();
      await load();
      return categories.value.length > 0 ? categories.value[0].id : '';
    } catch (e) {
      console.error(e);
      return '';
    }
  }

  async function reorder(ids: string[]) {
    try {
      await invokeReorderCategories(ids);
      await load();
    } catch (e) {
      console.error(e);
    }
  }

  return {
    categories,
    selectedCategoryId,
    loading,
    canAddMore,
    load,
    add,
    update,
    remove,
    select,
    ensureDefaultCategory,
    reorder,
    lockCategory,
    unlockCategory,
    toggleCategoryLock,
    isCategoryLocked,
  };
});