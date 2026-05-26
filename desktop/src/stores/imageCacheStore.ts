import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/**
 * 图片缓存 Store
 * 学习 Ditto 的两级缓存架构：
 * - imageCache：已加载的图片缩略图缓存
 * - noImageCache："无图片"标记缓存（避免重复检查）
 * - loadQueue：待加载队列（异步处理）
 */

export const useImageCacheStore = defineStore('imageCache', () => {
  // 缓存已加载的图片（id -> base64 data URL）
  const imageCache = ref<Map<string, string>>(new Map());

  // 缓存"无图片"标记（避免对纯文本条目反复查询）
  const noImageCache = ref<Set<string>>(new Set());

  // 待加载队列
  const loadQueue = ref<Array<{ id: string; path: string }>>([]);

  // 正在加载中的 ID（防止重复加载）
  const loadingIds = ref<Set<string>>(new Set());

  // 缓存大小限制（防止内存过大）
  const MAX_CACHE_SIZE = 200;
  const cacheSize = computed(() => imageCache.value.size);

  // 获取缓存的图片
  function getCachedImage(id: string): string | null {
    if (noImageCache.value.has(id)) return null;
    return imageCache.value.get(id) || null;
  }

  // 检查是否需要加载（不在缓存、不在无图片标记、不在加载中）
  function needsLoad(id: string): boolean {
    return !imageCache.value.has(id) &&
           !noImageCache.value.has(id) &&
           !loadingIds.value.has(id);
  }

  // 标记无图片
  function markNoImage(id: string) {
    noImageCache.value.add(id);
    imageCache.value.delete(id);
    loadingIds.value.delete(id);
  }

  // 缓存图片
  function cacheImage(id: string, base64: string) {
    // 缓存大小限制：超出时清理最旧的
    if (imageCache.value.size >= MAX_CACHE_SIZE) {
      // 删除最早的一半缓存
      const keys = Array.from(imageCache.value.keys());
      const toDelete = keys.slice(0, Math.floor(MAX_CACHE_SIZE / 2));
      for (const key of toDelete) {
        imageCache.value.delete(key);
      }
    }

    imageCache.value.set(id, base64);
    noImageCache.value.delete(id);
    loadingIds.value.delete(id);
  }

  // 加入加载队列
  function addToLoadQueue(id: string, path: string) {
    if (needsLoad(id)) {
      loadingIds.value.add(id);
      loadQueue.value.push({ id, path });
    }
  }

  // 批量加入加载队列
  function addToLoadQueueBatch(items: Array<{ id: string; path: string }>) {
    for (const item of items) {
      addToLoadQueue(item.id, item.path);
    }
  }

  // 处理一批加载请求（由外部定时器调用）
  async function processBatch(batchSize: number = 5): Promise<number> {
    if (loadQueue.value.length === 0) return 0;

    const batch = loadQueue.value.splice(0, batchSize);
    let loaded = 0;

    for (const item of batch) {
      try {
        const base64 = await invoke<string>('read_clipboard_image_file', { path: item.path });
        cacheImage(item.id, base64);
        loaded++;
      } catch (e) {
        // 文件不存在或读取失败，标记无图片
        markNoImage(item.id);
      }
    }

    return loaded;
  }

  // 清空缓存
  function clearCache() {
    imageCache.value.clear();
    noImageCache.value.clear();
    loadQueue.value = [];
    loadingIds.value.clear();
  }

  // 从缓存移除指定项
  function removeFromCache(id: string) {
    imageCache.value.delete(id);
    noImageCache.value.delete(id);
    loadingIds.value.delete(id);
  }

  return {
    imageCache,
    noImageCache,
    loadQueue,
    loadingIds,
    cacheSize,
    getCachedImage,
    needsLoad,
    markNoImage,
    cacheImage,
    addToLoadQueue,
    addToLoadQueueBatch,
    processBatch,
    clearCache,
    removeFromCache,
  };
});