import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { ClipboardCategory, ClipboardItem } from '../types';
import {
  getClipboardCategories, addClipboardCategory, updateClipboardCategory,
  deleteClipboardCategory, reorderClipboardCategories,
  getClipboardItems, addClipboardItem, updateClipboardItem,
  deleteClipboardItem, reorderClipboardItems
} from '../utils/db';
import { FREE_CATEGORY_LIMIT } from '../types';

export const useClipboardStore = defineStore('clipboard', () => {
  const categories = ref<ClipboardCategory[]>([]);
  const items = ref<ClipboardItem[]>([]);
  const selectedCategoryId = ref<string | null>(null);
  const loading = ref(false);
  const searchQuery = ref('');

  const canAddMore = computed(() => categories.value.length < FREE_CATEGORY_LIMIT);

  // 过滤后的项目
  const filteredItems = computed(() => {
    let list = [...items.value];

    if (selectedCategoryId.value) {
      list = list.filter(i => i.categoryId === selectedCategoryId.value);
    }

    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      list = list.filter(i =>
        i.title.toLowerCase().includes(q) ||
        i.content.toLowerCase().includes(q)
      );
    }

    list.sort((a, b) => a.sortOrder - b.sortOrder);
    return list;
  });

  async function load() {
    loading.value = true;
    try {
      const [cats, itms] = await Promise.all([
        getClipboardCategories(),
        getClipboardItems()
      ]);
      categories.value = cats;
      items.value = itms;
      if (!selectedCategoryId.value && cats.length > 0) {
        selectedCategoryId.value = cats[0].id;
      }
    } finally {
      loading.value = false;
    }
  }

  async function addCategory(name: string, color: string) {
    const cat = await addClipboardCategory(name, color);
    categories.value.push(cat);
    return cat;
  }

  async function updateCategory(category: ClipboardCategory) {
    category.createdAt = Date.now();
    await updateClipboardCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  async function removeCategory(id: string) {
    await deleteClipboardCategory(id);
    categories.value = categories.value.filter(c => c.id !== id);
    items.value = items.value.filter(i => i.categoryId !== id);
    if (selectedCategoryId.value === id) {
      selectedCategoryId.value = categories.value[0]?.id || null;
    }
  }

  function selectCategory(id: string | null) {
    selectedCategoryId.value = id;
  }

  async function reorderCategories(ids: string[]) {
    await reorderClipboardCategories(ids);
    await load();
  }

  async function addItem(itemData: Omit<ClipboardItem, 'id' | 'createdAt' | 'sortOrder'>) {
    const minSortOrder = items.value.length > 0
      ? Math.min(...items.value.map(i => i.sortOrder))
      : 0;
    const newSortOrder = minSortOrder - 1;

    const item = await addClipboardItem({
      ...itemData,
      sortOrder: newSortOrder,
    });
    items.value.unshift(item);
    return item;
  }

  async function updateItem(item: ClipboardItem) {
    await updateClipboardItem(item);
    const index = items.value.findIndex(i => i.id === item.id);
    if (index !== -1) items.value[index] = item;
  }

  async function removeItem(id: string) {
    await deleteClipboardItem(id);
    items.value = items.value.filter(i => i.id !== id);
  }

  async function reorderItems(ids: string[]) {
    await reorderClipboardItems(ids);
    await load();
  }

  // 从剪贴板粘贴
  async function pasteFromClipboard(message?: { success: (msg: string) => void; warning: (msg: string) => void }) {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        const title = text.length > 30 ? text.substring(0, 30) + '...' : text;
        await addItem({
          categoryId: selectedCategoryId.value || categories.value[0]?.id || '',
          title,
          content: text,
          priority: 2,
        });
        message?.success('已粘贴文本');
        return;
      }
    } catch (_) {}

    try {
      const clipItems = await navigator.clipboard.read();
      for (const item of clipItems) {
        if (item.types.includes('image/png') || item.types.includes('image/jpeg')) {
          const blob = await item.getType(item.types.find(t => t.startsWith('image/'))!);
          const reader = new FileReader();
          reader.onloadend = async () => {
            const base64 = reader.result as string;
            await addItem({
              categoryId: selectedCategoryId.value || categories.value[0]?.id || '',
              title: '剪贴板图片',
              content: '',
              imageBase64: base64,
              priority: 2,
            });
            message?.success('已粘贴图片');
          };
          reader.readAsDataURL(blob);
          return;
        }
      }
    } catch (_) {}

    message?.warning('剪贴板为空');
  }

  return {
    categories,
    items,
    selectedCategoryId,
    loading,
    searchQuery,
    canAddMore,
    filteredItems,
    load,
    addCategory,
    updateCategory,
    removeCategory,
    selectCategory,
    reorderCategories,
    addItem,
    updateItem,
    removeItem,
    reorderItems,
    pasteFromClipboard,
  };
});
