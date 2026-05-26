/**
 * 图片后台加载服务
 * 学习 Ditto 的异步加载架构：
 * 定时处理加载队列，不阻塞主线程渲染
 */

import { useImageCacheStore } from '../stores/imageCacheStore';

let loadingInterval: number | null = null;
const BATCH_SIZE = 5; // 每批处理 5 个
const INTERVAL_MS = 100; // 100ms 间隔

/**
 * 启动后台图片加载服务
 */
export function startImageLoader() {
  if (loadingInterval !== null) return; // 已启动

  loadingInterval = window.setInterval(async () => {
    const imageCacheStore = useImageCacheStore();

    if (imageCacheStore.loadQueue.length === 0) return;

    // 处理一批加载请求
    await imageCacheStore.processBatch(BATCH_SIZE);
  }, INTERVAL_MS);
}

/**
 * 停止后台图片加载服务
 */
export function stopImageLoader() {
  if (loadingInterval !== null) {
    window.clearInterval(loadingInterval);
    loadingInterval = null;
  }
}

/**
 * 重启后台图片加载服务
 */
export function restartImageLoader() {
  stopImageLoader();
  startImageLoader();
}