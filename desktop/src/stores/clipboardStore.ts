import { defineStore } from 'pinia';
import { ref, shallowRef, computed, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ClipboardCategory, ClipboardItem } from '../types';
import {
  getClipboardCategories, addClipboardCategory, updateClipboardCategory,
  deleteClipboardCategory, reorderClipboardCategories,
  getClipboardItemsPaginated, getClipboardItemsCount,
  addClipboardItem, updateClipboardItem,
  deleteClipboardItem, reorderClipboardItems
} from '../utils/db';
import { FREE_CATEGORY_LIMIT, BUILTIN_CLIPBOARD_CATEGORY_META, BUILTIN_CLIPBOARD_CATEGORIES, isBuiltinClipboardCategory } from '../types';
import { useSettingsStore } from './settingsStore';

// 分页加载配置
const PAGE_SIZE = 50; // 每次加载 50 条

export const useClipboardStore = defineStore('clipboard', () => {
  const categories = ref<ClipboardCategory[]>([]);
  // 用 shallowRef 减少 Vue 深度响应追踪开销（切换分类时减少重渲染）
  const items = shallowRef<ClipboardItem[]>([]);
  const selectedCategoryId = ref<string | null>(null);
  const loading = ref(false);
  const searchQuery = ref('');

  // 分页加载状态
  const totalItemsCount = ref(0);
  const loadedOffset = ref(0);
  const hasMore = computed(() => loadedOffset.value < totalItemsCount.value);

  // 从 settingsStore 同步排序模式
  const sortMode = computed(() => {
    const settingsStore = useSettingsStore();
    return settingsStore.settings.clipboardSortMode || 'custom';
  });

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

  // 预缓存：按分类分组的已排序 items（启动时预计算，切换分类零延迟）
  const precomputedCategoryItems = ref<Map<string, ClipboardItem[]>>(new Map());
  const precomputedAllItems = shallowRef<ClipboardItem[]>([]);

  // 预计算函数：根据当前排序模式，计算每个分类的已排序列表
  // 使用分片计算，避免阻塞主线程
  function precomputeAllCategories() {
    const mode = sortMode.value;
    const allItems = [...items.value];

    // 排序函数
    const sortFn = (a: ClipboardItem, b: ClipboardItem) => {
      if (mode === 'name') return a.title.localeCompare(b.title, 'zh');
      if (mode === 'createdAt') return b.createdAt - a.createdAt;
      return a.sortOrder - b.sortOrder;
    };

    // 如果数据量小于 100，直接在主线程计算（足够快）
    if (allItems.length < 100) {
      allItems.sort(sortFn);
      precomputedAllItems.value = allItems;

      const categoryMap = new Map<string, ClipboardItem[]>();
      for (const item of items.value) {
        const catId = item.categoryId;
        if (!categoryMap.has(catId)) categoryMap.set(catId, []);
        categoryMap.get(catId)!.push(item);
      }
      for (const catItems of categoryMap.values()) {
        catItems.sort(sortFn);
      }
      precomputedCategoryItems.value = categoryMap;
      return;
    }

    // 数据量大时，使用 setTimeout 分片计算
    const chunkSize = 50;
    let currentIndex = 0;
    const categoryMap = new Map<string, ClipboardItem[]>();

    // 先快速分组（不需要排序）
    for (const item of items.value) {
      const catId = item.categoryId;
      if (!categoryMap.has(catId)) categoryMap.set(catId, []);
      categoryMap.get(catId)!.push(item);
    }

    // 分片排序全量列表
    const sortChunk = () => {
      const end = Math.min(currentIndex + chunkSize, allItems.length);
      // 对当前 chunk 进行局部排序（实际上是整体排序，但分多次执行）
      if (currentIndex === 0) {
        // 第一次 chunk 执行整体排序
        allItems.sort(sortFn);
        precomputedAllItems.value = allItems;
      }

      currentIndex = end;

      if (currentIndex < allItems.length) {
        // 还有更多数据，继续下一 chunk
        setTimeout(sortChunk, 0);
      } else {
        // 全量排序完成，现在排序各分类
        for (const catItems of categoryMap.values()) {
          catItems.sort(sortFn);
        }
        precomputedCategoryItems.value = categoryMap;
      }
    };

    // 启动分片计算
    setTimeout(sortChunk, 0);
  }

  // 过滤后的项目（直接使用预计算结果，零延迟）
  const filteredItems = computed(() => {
    // 直接从预计算结果取值
    if (selectedCategoryId.value) {
      return precomputedCategoryItems.value.get(selectedCategoryId.value) || [];
    } else {
      return precomputedAllItems.value;
    }
  });

  async function load() {
    loading.value = true;
    try {
      // 加载分类
      const cats = await getClipboardCategories();

      // 迁移：将旧的同名分类替换为内置分类
      const existingIds = new Set(cats.map(c => c.id));
      const idsToDelete: string[] = [];

      for (const meta of BUILTIN_CLIPBOARD_CATEGORY_META) {
        if (!existingIds.has(meta.id)) {
          const oldCat = cats.find(c => c.name === meta.name && !isBuiltinClipboardCategory(c.id));
          if (oldCat) {
            idsToDelete.push(oldCat.id);
          } else {
            await addClipboardCategory(meta.name, meta.color);
          }
        }
      }

      for (const id of idsToDelete) {
        await deleteClipboardCategory(id);
      }

      // 分页加载项目：只加载最近 50 条
      const [finalCats, paginatedItems, totalCount] = await Promise.all([
        getClipboardCategories(),
        getClipboardItemsPaginated(PAGE_SIZE, 0),
        getClipboardItemsCount()
      ]);

      categories.value = finalCats;
      items.value = paginatedItems;
      totalItemsCount.value = totalCount;
      loadedOffset.value = paginatedItems.length;

      // 预计算所有分类的已排序列表（启动时一次性计算）
      precomputeAllCategories();
    } finally {
      loading.value = false;
    }
    // 注册剪贴板变化监听（只注册一次）
    initClipboardListener();
  }

  // 加载更多项目（滚动到底部时调用）
  async function loadMore() {
    if (loading.value || !hasMore.value) return;

    loading.value = true;
    try {
      const moreItems = await getClipboardItemsPaginated(PAGE_SIZE, loadedOffset.value);
      // 追加到现有列表
      items.value = [...items.value, ...moreItems];
      loadedOffset.value += moreItems.length;

      // 重新预计算（数据增加了）
      precomputeAllCategories();
    } finally {
      loading.value = false;
    }
  }

  // 监听排序模式变化，重新预计算
  watch(sortMode, () => {
    precomputeAllCategories();
  });
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

  // 锁定/解锁分类
  async function lockCategory(category: ClipboardCategory): Promise<void> {
    category.locked = true;
    await updateClipboardCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  async function unlockCategory(category: ClipboardCategory): Promise<void> {
    category.locked = false;
    await updateClipboardCategory(category);
    const index = categories.value.findIndex(c => c.id === category.id);
    if (index !== -1) categories.value[index] = category;
  }

  async function toggleCategoryLock(category: ClipboardCategory): Promise<'locked' | 'unlocked'> {
    if (category.locked) {
      await unlockCategory(category);
      return 'unlocked';
    } else {
      await lockCategory(category);
      return 'locked';
    }
  }

  // 检查卡片是否在锁定分类下
  function isItemInLockedCategory(item: ClipboardItem): boolean {
    const category = categories.value.find(c => c.id === item.categoryId);
    return category?.locked === true;
  }

  // 检查卡片是否锁定（卡片级别或分类级别）
  function isItemLocked(item: ClipboardItem): boolean {
    return item.locked === true || isItemInLockedCategory(item);
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
    // shallowRef 需要整体替换触发更新
    items.value = [item, ...items.value];
    return item;
  }

  async function updateItem(item: ClipboardItem) {
    await updateClipboardItem(item);
    // shallowRef 需要整体替换触发更新
    items.value = items.value.map(i => i.id === item.id ? { ...item } : i);
    // 重新预计算排序（分类可能变化）
    precomputeAllCategories();
  }

  async function removeItems(ids: string[]) {
    // 使用批量删除命令（事务化，一次提交）
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('batch_delete_clipboard_items', { ids });
    items.value = items.value.filter(i => !ids.includes(i.id));
    // 重新预计算排序
    precomputeAllCategories();
  }

  async function removeItem(id: string) {
    await deleteClipboardItem(id);
    items.value = items.value.filter(i => i.id !== id);
    // 重新预计算排序
    precomputeAllCategories();
  }

  // 批量更新项目分类（事务化，一次提交）
  async function batchUpdateItemsCategory(ids: string[], categoryId: string): Promise<number> {
    const { invoke } = await import('@tauri-apps/api/core');
    const count = await invoke<number>('batch_update_clipboard_items_category', { ids, categoryId });
    // shallowRef 需要整体替换触发更新
    const idsSet = new Set(ids);
    items.value = items.value.map(i => idsSet.has(i.id) ? { ...i, categoryId } : i);
    return count;
  }

  async function reorderItems(ids: string[]) {
    await reorderClipboardItems(ids);
    await load();
  }

  // 设置项目过期时间
  async function setItemExpiry(id: string, expiresAt: number | null): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_clipboard_item_expiry', { id, expiresAt });
    // shallowRef 需要整体替换触发更新
    items.value = items.value.map(i => i.id === id ? { ...i, expiresAt } : i);
  }

  // 清理已过期项目
  async function cleanupExpiredItems(): Promise<number> {
    const { invoke } = await import('@tauri-apps/api/core');
    const count = await invoke<number>('cleanup_expired_items');
    await load();
    return count;
  }

  // 清空所有未锁定的剪贴板项
  async function clearAllUnlocked(): Promise<number> {
    const { invoke } = await import('@tauri-apps/api/core');
    const count = await invoke<number>('clear_all_unlocked_clipboard_items');
    await load();
    return count;
  }

  // 锁定/解锁项目
  async function lockItem(item: ClipboardItem): Promise<void> {
    item.locked = true;
    await updateClipboardItem(item);
    // shallowRef 需要整体替换触发更新
    items.value = items.value.map(i => i.id === item.id ? item : i);
  }

  async function unlockItem(item: ClipboardItem): Promise<void> {
    item.locked = false;
    await updateClipboardItem(item);
    // shallowRef 需要整体替换触发更新
    items.value = items.value.map(i => i.id === item.id ? item : i);
  }

  // 切换锁定状态
  async function toggleItemLock(item: ClipboardItem): Promise<'locked' | 'unlocked'> {
    if (item.locked) {
      await unlockItem(item);
      return 'unlocked';
    } else {
      await lockItem(item);
      return 'locked';
    }
  }

  // 移动项目到最顶部
  async function moveItemToTop(item: ClipboardItem): Promise<void> {
    const categoryItems = items.value.filter(i => i.categoryId === item.categoryId);
    const minSort = Math.min(...categoryItems.map(i => i.sortOrder));
    item.sortOrder = (minSort || 0) - 1;
    await updateClipboardItem(item);
    // shallowRef 需要整体替换触发更新
    items.value = items.value.map(i => i.id === item.id ? { ...i, sortOrder: item.sortOrder } : i);
    // 重新预计算排序
    precomputeAllCategories();
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
        // 文本默认3天过期
        const expiresAt = Date.now() + (3 * 24 * 60 * 60 * 1000);
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
          // 图片默认1小时过期（减少图片堆积）
          const expiresAt = Date.now() + (1 * 60 * 60 * 1000);
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
          // shallowRef 需要整体替换触发更新
          items.value = [newItem, ...items.value];
          message?.success('已粘贴图片');
          return;
        }
      }
    } catch (_) {}

    message?.warning('剪贴板为空');
  }

  // 设置排序模式
  async function setSortMode(mode: 'custom' | 'name' | 'createdAt') {
    const settingsStore = useSettingsStore();

    if (mode !== 'custom' && sortMode.value === 'custom') {
      // 切换到自动排序前，备份当前的 sortOrder
      const backup: Record<string, number> = {};
      items.value.forEach(i => { backup[i.id] = i.sortOrder; });
      settingsStore.setClipboardSortBackup(backup);
    }

    if (mode === 'custom' && sortMode.value !== 'custom') {
      // 切换回自定义排序时，从备份恢复 sortOrder
      const backup = settingsStore.settings.clipboardSortBackup;
      if (backup) {
        for (const item of items.value) {
          if (backup[item.id] !== undefined) {
            item.sortOrder = backup[item.id];
            await updateClipboardItem(item);
          }
        }
        await load();
      }
    }

    settingsStore.setClipboardSortMode(mode);
  }

  return {
    categories,
    items,
    selectedCategoryId,
    loading,
    searchQuery,
    sortMode,
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
    removeItems,
    reorderItems,
    pasteFromClipboard,
    setItemExpiry,
    cleanupExpiredItems,
    clearAllUnlocked,
    lockItem,
    unlockItem,
    toggleItemLock,
    lockCategory,
    unlockCategory,
    toggleCategoryLock,
    isItemInLockedCategory,
    isItemLocked,
    setSortMode,
    moveItemToTop,
    batchUpdateItemsCategory,
    // 分页加载
    hasMore,
    loadMore,
  };
});
