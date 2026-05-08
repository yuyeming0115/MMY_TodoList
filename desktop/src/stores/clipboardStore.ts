import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ClipboardCategory, ClipboardItem } from '../types';
import {
  getClipboardCategories, addClipboardCategory, updateClipboardCategory,
  deleteClipboardCategory, reorderClipboardCategories,
  getClipboardItems, addClipboardItem, updateClipboardItem,
  deleteClipboardItem, reorderClipboardItems
} from '../utils/db';
import { FREE_CATEGORY_LIMIT, BUILTIN_CLIPBOARD_CATEGORY_META, BUILTIN_CLIPBOARD_CATEGORIES, isBuiltinClipboardCategory } from '../types';

export const useClipboardStore = defineStore('clipboard', () => {
  const categories = ref<ClipboardCategory[]>([]);
  const items = ref<ClipboardItem[]>([]);
  const selectedCategoryId = ref<string | null>(null);
  const loading = ref(false);
  const searchQuery = ref('');

  // 内置分类（始终存在，不可删除）
  const builtinCategories = computed(() =>
    categories.value.filter(c => isBuiltinClipboardCategory(c.id))
  );

  // 用户自定义分类
  const customCategories = computed(() =>
    categories.value.filter(c => !isBuiltinClipboardCategory(c.id))
  );

  // 可添加的自定义分类数量
  const canAddMore = computed(() => customCategories.value.length < FREE_CATEGORY_LIMIT);

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

      // 迁移：将旧的同名分类替换为内置分类
      const existingIds = new Set(cats.map(c => c.id));
      const now = Date.now();
      const idsToDelete: string[] = [];

      for (const meta of BUILTIN_CLIPBOARD_CATEGORY_META) {
        if (!existingIds.has(meta.id)) {
          // 查找是否有同名的旧分类
          const oldCat = cats.find(c => c.name === meta.name && !isBuiltinClipboardCategory(c.id));
          if (oldCat) {
            // 迁移：把旧分类的项目转移到内置分类
            for (const item of itms) {
              if (item.categoryId === oldCat.id) {
                item.categoryId = meta.id;
                await updateClipboardItem(item);
              }
            }
            // 标记删除旧分类
            idsToDelete.push(oldCat.id);
          } else {
            // 没有旧分类，直接创建内置分类
            const builtinCat: ClipboardCategory = {
              id: meta.id,
              name: meta.name,
              color: meta.color,
              sortOrder: meta.sortOrder,
              createdAt: now,
            };
            await addClipboardCategory(builtinCat.name, builtinCat.color);
            cats.push(builtinCat);
          }
        }
      }

      // 删除旧的重复分类
      for (const id of idsToDelete) {
        await deleteClipboardCategory(id);
      }

      // 重新加载以获取最新数据
      const [finalCats, finalItms] = await Promise.all([
        getClipboardCategories(),
        getClipboardItems()
      ]);

      categories.value = finalCats;
      items.value = finalItms;
      // 默认不选中任何分类，显示全部项目
    } finally {
      loading.value = false;
    }
    // 注册剪贴板变化监听（只注册一次）
    initClipboardListener();
  }

  // 监听后端剪贴板变化事件
  let clipboardUnlisten: UnlistenFn | null = null;

  async function initClipboardListener() {
    if (clipboardUnlisten) return; // 已注册过
    clipboardUnlisten = await listen('clipboard-changed', async () => {
      await load();
    });
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
    if (isBuiltinClipboardCategory(id)) {
      return;
    }
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
    // 去重：相同 category + 相同内容/图片则跳过
    const exists = items.value.some(i =>
      i.categoryId === itemData.categoryId &&
      (itemData.imageBase64
        ? i.imageBase64 === itemData.imageBase64
        : i.content === itemData.content)
    );
    if (exists) return null;

    const minSortOrder = items.value.length > 0
      ? Math.min(...items.value.map(i => i.sortOrder))
      : 0;
    const newSortOrder = minSortOrder - 1;

    const item = await addClipboardItem({
      ...itemData,
      sortOrder: newSortOrder,
      expiresAt: itemData.expiresAt || null,
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

  // 设置项目过期时间
  async function setItemExpiry(id: string, expiresAt: number | null): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_clipboard_item_expiry', { id, expiresAt });
    const index = items.value.findIndex(i => i.id === id);
    if (index !== -1) items.value[index].expiresAt = expiresAt;
  }

  // 清理已过期项目
  async function cleanupExpiredItems(): Promise<number> {
    const { invoke } = await import('@tauri-apps/api/core');
    const count = await invoke<number>('cleanup_expired_items');
    await load();
    return count;
  }

  // 收藏/取消收藏项目，返回操作结果：'favorited' | 'unfavorited' | 'error'
  async function favoriteItem(item: ClipboardItem): Promise<'favorited' | 'unfavorited' | 'error'> {
    const favoriteCat = categories.value.find(c => c.id === BUILTIN_CLIPBOARD_CATEGORIES.FAVORITE);
    if (!favoriteCat) return 'error';

    if (item.categoryId === favoriteCat.id) {
      // 已经在收藏中，取消收藏（移动到文本分类）
      const textCat = categories.value.find(c => c.id === BUILTIN_CLIPBOARD_CATEGORIES.TEXT);
      if (!textCat) return 'error';
      item.categoryId = textCat.id;
      // 恢复默认30天过期
      item.expiresAt = Date.now() + (30 * 24 * 60 * 60 * 1000);
      await updateClipboardItem(item);
      const index = items.value.findIndex(i => i.id === item.id);
      if (index !== -1) items.value[index] = item;
      return 'unfavorited';
    } else {
      // 移动到收藏分类，清除过期时间（收藏永不过期）
      item.categoryId = favoriteCat.id;
      item.expiresAt = null;
      await updateClipboardItem(item);
      const index = items.value.findIndex(i => i.id === item.id);
      if (index !== -1) items.value[index] = item;
      return 'favorited';
    }
  }

  // 从剪贴板粘贴
  async function pasteFromClipboard(message?: { success: (msg: string) => void; warning: (msg: string) => void }) {
    const textCategoryId = categories.value.find(c => c.id === BUILTIN_CLIPBOARD_CATEGORIES.TEXT)?.id
      || categories.value.find(c => isBuiltinClipboardCategory(c.id))?.id
      || categories.value[0]?.id || '';
    const imageCategoryId = categories.value.find(c => c.id === BUILTIN_CLIPBOARD_CATEGORIES.IMAGE)?.id
      || categories.value.find(c => isBuiltinClipboardCategory(c.id))?.id
      || categories.value[0]?.id || '';

    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        const title = text.length > 30 ? text.substring(0, 30) + '...' : text;
        // 文本默认30天过期
        const expiresAt = Date.now() + (30 * 24 * 60 * 60 * 1000);
        const result = await addItem({
          categoryId: textCategoryId,
          title,
          content: text,
          priority: 2,
          expiresAt,
        });
        if (result) {
          message?.success('已粘贴文本');
        } else {
          message?.warning('内容已存在');
        }
        return;
      }
    } catch (_) {}

    try {
      const clipItems = await navigator.clipboard.read();
      for (const item of clipItems) {
        if (item.types.includes('image/png') || item.types.includes('image/jpeg')) {
          const blob = await item.getType(item.types.find(t => t.startsWith('image/'))!);
          const base64 = await new Promise<string>((resolve) => {
            const reader = new FileReader();
            reader.onloadend = () => resolve(reader.result as string);
            reader.readAsDataURL(blob);
          });

          // 图片通过后端存文件，前端只传 base64
          const { invoke } = await import('@tauri-apps/api/core');
          const minSortOrder = items.value.length > 0
            ? Math.min(...items.value.map(i => i.sortOrder))
            : 0;
          // 图片默认7天过期
          const expiresAt = Date.now() + (7 * 24 * 60 * 60 * 1000);
          const newItem = await invoke<any>('add_clipboard_item', {
            item: {
              categoryId: imageCategoryId,
              title: '剪贴板图片',
              content: '',
              imageBase64: base64,
              priority: 2,
              sortOrder: minSortOrder - 1,
              expiresAt,
            },
          });
          items.value.unshift(newItem);
          message?.success('已粘贴图片');
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
    builtinCategories,
    customCategories,
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
    favoriteItem,
    setItemExpiry,
    cleanupExpiredItems,
  };
});
