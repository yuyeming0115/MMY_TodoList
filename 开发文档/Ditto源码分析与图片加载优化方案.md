# Ditto 剪贴板源码分析与改进方案

> 分析时间：2026-05-25
> 源码来源：https://github.com/sabrogden/Ditto

---

## 一、Ditto 核心架构分析

### 1.1 数据库结构

Ditto 使用 SQLite 存储所有剪贴板数据：

```sql
-- Main 表：元数据
CREATE TABLE Main(
    lID INTEGER PRIMARY KEY AUTOINCREMENT,
    lDate INTEGER,           -- 时间戳
    mText TEXT,              -- 描述文本
    lShortCut INTEGER,       -- 快捷键
    lDontAutoDelete INTEGER, -- 不自动删除标记
    CRC INTEGER,             -- CRC校验（用于去重）
    bIsGroup INTEGER,        -- 是否为分组
    lParentID INTEGER,       -- 父分组ID
    clipOrder REAL,          -- 排序值
    ...
);

-- Data 表：实际剪贴板数据
CREATE TABLE Data(
    lID INTEGER PRIMARY KEY AUTOINCREMENT,
    lParentID INTEGER,       -- 关联到 Main.lID
    strClipBoardFormat TEXT, -- 格式类型（CF_DIB, CF_TEXT, PNG等）
    ooData BLOB              -- 二进制数据（图片直接存这里）
);
```

**关键设计**：
- Main 表只存元数据，Data 表存实际数据
- 通过 `lParentID` 关联，一对多关系（一个 Clip 可有多种格式）
- 图片数据直接存入 BLOB，不使用外部文件

### 1.2 图片加载架构（核心优化点）

Ditto 使用 **异步加载 + 两级缓存** 架构：

```
┌─────────────────────────────────────────────────────────────┐
│                     主线程（UI渲染）                          │
│  ┌─────────────┐                                            │
│  │ LVN_GETDISPINFO │ ──请求图片数据──► GetDispInfo 处理      │
│  └─────────────┘                                            │
└         │                                                    │
         ▼                                                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    缓存检查                               ││
│  │  m_cf_dibCache ────► 有缓存 ────► 直接返回                ││
│  │  m_cf_NO_dibCache ─► 无图片标记 ─► 跳过                   ││
│  │  都不在 ──────────► 加入加载队列                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼ 加入队列
┌─────────────────────────────────────────────────────────────┐
│              异步加载线程（m_extraDataThread）                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 1. 从数据库加载原图 BLOB                                  ││
│  │ 2. GDI+ 转换为缩略图（按显示高度）                         ││
│  │ 3. 释放原图数据（节省内存）                                ││
│  │ 4. 缩略图存入 m_cf_dibCache                               ││
│  │ 5. 通知主线程刷新显示                                      ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**关键代码片段**：

```cpp
// QPasteWnd.cpp - GetDispInfo 处理图片请求
if (pItem->mask & LVIF_CF_DIB && CGetSetOptions::m_bDrawThumbnail)
{
    // 1. 检查"无图片"缓存，避免重复查询
    CF_NoDibTypeMap::iterator iterNoDib = m_cf_NO_dibCache.find(itemId);
    if (iterNoDib == m_cf_NO_dibCache.end())
    {
        // 2. 检查图片缓存
        CF_DibTypeMap::iterator iterDib = m_cf_dibCache.find(itemId);
        if (iterDib == m_cf_dibCache.end())
        {
            // 3. 都不在缓存，加入异步加载队列
            m_ExtraDataLoadItems.push_back(format);
            m_extraDataThread.FireLoadExtraData(rowHeight);
        }
        else
        {
            // 有缓存，直接返回
            pItem->lParam = (LPARAM)&(iterDib->second);
        }
    }
}
```

```cpp
// ClipFormatQListCtrl.cpp - 缩略图生成（只转换一次）
HGLOBAL GetDibFittingToHeight(CDC *pDc, int height)
{
    if(m_convertedToSmallImage)
        return m_hgData;  // 已转换过，直接返回缓存的缩略图

    m_convertedToSmallImage = true;

    // 原图 → 缩略图
    CBitmapHelper::GetCBitmap(this, pDc, &Bitmap, height);

    // 释放原图，只保留缩略图
    this->Free();  // 关键：节省内存
    this->m_hgData = CBitmapHelper::hBitmapToDIB(...);

    return this->m_hgData;
}
```

### 1.3 缩略图缓存策略

| 缓存类型 | 作用 | 数据结构 |
|---------|------|----------|
| `m_cf_dibCache` | 已加载的图片缩略图 | `Map<ClipID, CClipFormat>` |
| `m_cf_NO_dibCache` | "无图片"标记 | `Map<ClipID, bool>` |
| `m_ExtraDataLoadItems` | 待加载队列 | `List<CClipFormat>` |

**设计目的**：
- `m_cf_dibCache`：避免重复加载同一图片
- `m_cf_NO_dibCache`：避免对纯文本条目反复查询数据库
- 异步队列：不阻塞 UI，按需加载

---

## 二、当前实现对比

### 2.1 架构对比

| 特性 | Ditto | 当前实现 |
|------|-------|----------|
| **数据存储** | SQLite BLOB（图片存数据库） | 文件 + SQLite 路径 |
| **图片加载** | 异步线程，不阻塞 UI | 同步加载（watch 触发） |
| **缩略图缓存** | 内存缓存，一次转换 | 无缓存，每次从文件读取 |
| **无图片检查** | 专用缓存标记跳过 | 每次检查 imageBase64/imagePath |
| **去重机制** | CRC 校验 | 内容/base64 比对 |

### 2.2 性能瓶颈分析

当前实现的瓶颈：
1. **图片加载同步**：每次显示图片都要调用 `invoke('read_clipboard_image_file')`
2. **无缓存**：滚动时同一图片反复读取文件
3. **无异步队列**：可视区域图片一次性全部加载

---

## 三、改进方案

### 3.1 Phase 1：图片内存缓存（已完成）

已完成：
- 移除 `thumbnail_base64` 数据库字段
- 图片存储为独立文件
- ClipboardItemCard.vue 从文件动态读取

### 3.2 Phase 2：异步加载 + 两级缓存

**目标**：实现 Ditto 级别的图片加载性能

**实施步骤**：

#### Step 1：创建图片缓存 Store

```typescript
// stores/imageCacheStore.ts
export const useImageCacheStore = defineStore('imageCache', () => {
  // 缓存已加载的图片
  const imageCache = ref<Map<string, string>>(new Map());
  // 缓存"无图片"标记
  const noImageCache = ref<Set<string>>(new Set());
  // 待加载队列
  const loadQueue = ref<Array<{ id: string, path: string }>>([]);

  // 获取缓存的图片
  function getCachedImage(id: string): string | null {
    if (noImageCache.value.has(id)) return null;
    return imageCache.value.get(id) || null;
  }

  // 标记无图片
  function markNoImage(id: string) {
    noImageCache.value.add(id);
  }

  // 缓存图片
  function cacheImage(id: string, base64: string) {
    imageCache.value.set(id, base64);
    noImageCache.value.delete(id); // 移除无图片标记
  }

  // 加入加载队列
  function addToLoadQueue(id: string, path: string) {
    if (!imageCache.value.has(id) && !noImageCache.value.has(id)) {
      loadQueue.value.push({ id, path });
    }
  }

  return {
    getCachedImage,
    markNoImage,
    cacheImage,
    addToLoadQueue,
    loadQueue,
  };
});
```

#### Step 2：修改 ClipboardItemCard.vue 使用缓存

```typescript
// ClipboardItemCard.vue
import { useImageCacheStore } from '../stores/imageCacheStore';

const imageCacheStore = useImageCacheStore();

// 从缓存或文件加载图片
async function loadImageFromCache() {
  // 1. 先检查缓存
  const cached = imageCacheStore.getCachedImage(props.item.id);
  if (cached) {
    imageSrc.value = cached;
    return;
  }

  // 2. 缓存不存在，加入加载队列（异步）
  if (props.item.imagePath) {
    imageCacheStore.addToLoadQueue(props.item.id, props.item.imagePath);
  }
}
```

#### Step 3：创建后台加载 Worker

```typescript
// utils/imageLoader.ts
export function startImageLoader() {
  const cacheStore = useImageCacheStore();

  // 定时检查队列并加载
  setInterval(async () => {
    if (cacheStore.loadQueue.length === 0) return;

    const batch = cacheStore.loadQueue.splice(0, 5); // 每批5个
    for (const item of batch) {
      try {
        const base64 = await invoke<string>('read_clipboard_image_file', { path: item.path });
        cacheStore.cacheImage(item.id, base64);
      } catch {
        cacheStore.markNoImage(item.id);
      }
    }
  }, 100); // 100ms 间隔
}
```

### 3.3 Phase 3：预加载优化

可视区域预加载，非可视区域延迟：

```typescript
// ClipboardPanel.vue
// 监听滚动，预加载可视区域图片
function preloadVisibleImages() {
  const visibleItems = getVisibleItems();
  for (const item of visibleItems) {
    if (item.imagePath) {
      imageCacheStore.addToLoadQueue(item.id, item.imagePath);
    }
  }
}
```

---

## 四、实施计划

| 阶段 | 任务 | 预估时间 |
|------|------|----------|
| Phase 2.1 | 创建 imageCacheStore.ts | 30分钟 |
| Phase 2.2 | 修改 ClipboardItemCard.vue 使用缓存 | 20分钟 |
| Phase 2.3 | 实现后台加载队列 | 30分钟 |
| Phase 2.4 | 测试验证性能提升 | 15分钟 |
| Phase 3 | 预加载优化 | 20分钟 |

---

## 五、预期效果

- **滚动流畅**：图片从缓存直接读取，无 IO 等待
- **首次加载快**：异步加载不阻塞渲染，先显示占位符
- **内存可控**：两级缓存避免重复加载，无图片条目跳过检查
- **接近 Ditto 性能**：大量图片时不再卡顿

---

## 六、变更记录

| 日期 | 内容 |
|------|------|
| 2026-05-25 | 创建文档，完成 Ditto 源码分析 |
| 2026-05-25 | Phase 2 完成：实现两级缓存 + 异步加载服务 |
| 2026-05-25 | Phase 3 完成：可视区域预加载 + 分类切换预加载 |

### 已完成代码文件

| 文件 | 作用 |
|------|------|
| `src/stores/imageCacheStore.ts` | 图片缓存 Store（两级缓存架构） |
| `src/utils/imageLoader.ts` | 后台异步加载服务（100ms定时，每批5条） |
| `src/components/ClipboardItemCard.vue` | 使用缓存加载图片，监听缓存更新 |
| `src/components/ClipboardPanel.vue` | 启动加载服务、滚动预加载、分类切换预加载 |

### 优化效果总结

| 场景 | 优化前 | 优化后 |
|------|--------|--------|
| 图片滚动 | 每次从文件读取，卡顿 | 缓存命中，流畅 |
| 切换分类 | 首次加载慢 | 预加载前50条 |
| 重复浏览 | 反复读取同一文件 | 缓存直接返回 |
| 无图片条目 | 每次检查文件 | noImageCache跳过 |