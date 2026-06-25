# UI 布局重构 + 任务计时器功能

> 分支：`feature/ui-refactor-timer`
> 创建时间：2026-06-25
> 原型参考：项目根目录 `ui-prototype-v2.html`

---

## 一、重构目标

### 1.1 现有问题

1. **布局问题**：
   - Windows 端独占顶部标题栏（3个窗口按钮占一整行），Mac 端没有，两端体验不一致
   - 侧边栏按钮混排（置顶、任务、剪贴板、精简、主题、语言、设置），没有视觉分组
   - 任务卡1330行单文件太臃肿，需要组件拆分
   - hover scale 动画导致布局跳动
   - 优先级逻辑反直觉（1=高=3星）

2. **功能缺失**：
   - 没有任务计时器功能
   - 底部添加入口在任务多时滚动不到

### 1.2 新布局结构

```
┌─────────┬──────────────────────────────────────────┐
│ ● ● ●   │  [全部][工作][个人][+]  🔍搜索  ⚙️排序  │ ← 分类+工具栏合并一行
│         │                                          │
│ ─────── │ ┌──────────────────────────────────────┐ │
│ 📋 任务 │ │                                      │ │
│ ⏱️ 计时 │ │        内容区（任务/计时器/剪贴板）    │ │
│ 📋 剪贴 │ │                                      │ │ ← 右下浮动 ➕ 按钮(FAB)
│         │ │                                      │ │
│ ─────── │ │                                      │ │
│ 📌 置顶 │ │                                      │ │
│ ⚡ 精简 │ └──────────────────────────────────────┘ │
│ 🌙 主题 │  待办5  已完成1        点击卡片展开·双击编辑│ ← 状态栏
│ 🌐 语言 │                                          │
│ ⚙️ 设置 │                                          │
└─────────┴──────────────────────────────────────────┘
  侧边栏           主内容区
```

**核心改动**：
- 窗口控制（红黄绿/最小化最大化关闭）统一放到侧边栏顶部，去掉 Windows 独占标题栏
- 侧边栏三段式分组：窗口控制 / 模块切换（带分隔线）/ 工具设置（带分隔线）
- 新增第三个模块「⏱️ 计时器」
- 分类Tab和工具栏合并为更紧凑的一行
- 右下角浮动添加按钮（FAB），始终可见

---

## 二、组件拆分计划

### 2.1 新增组件

| 组件 | 路径 | 说明 |
|------|------|------|
| `TimerPanel.vue` | `src/components/TimerPanel.vue` | 计时器面板主容器 |
| `TimerClock.vue` | `src/components/TimerClock.vue` | SVG圆环时钟表盘 + 倒计时数字 |
| `TimerQuickStart.vue` | `src/components/TimerQuickStart.vue` | 快速启动横滑卡片 |
| `TimerTaskList.vue` | `src/components/TimerTaskList.vue` | 全部计时任务列表 |
| `FixedReminders.vue` | `src/components/FixedReminders.vue` | 固定时间提醒（饭点/下班） |
| `FloatingButton.vue` | `src/components/FloatingButton.vue` | 右下角浮动添加按钮 |

### 2.2 重构组件

| 组件 | 改动 |
|------|------|
| `Home.vue` | 重构布局：侧边栏三段式、统一窗口控制、新增timer面板、FAB |
| `TaskCard.vue` | 重写：左侧优先级色条、统一标签对齐、hover无scale、展开/折叠、附件指示器 |
| `CategoryTabs.vue` | 改为圆角胶囊(pill)样式 |
| `SearchBar.vue` | 适配新布局样式 |
| `main.css` | 新增CSS变量和全局样式 |

### 2.3 保持不变

| 组件 | 说明 |
|------|------|
| `ClipboardPanel.vue` | 剪贴板面板功能保持，仅适配新样式 |
| `ClipboardItemCard.vue` | 剪贴板卡片暂不改（复制粘贴卡暂时不管） |
| `SettingsPage.vue` | 设置页面保持 |
| stores 层 | 数据层逻辑基本不变，新增timer store |
| Rust 后端 | 计时器通知可能需要新增command，但计时器核心逻辑前端实现 |

---

## 三、数据模型

### 3.1 计时器相关类型（新增到 `types/index.ts`）

```typescript
export interface TimerTask {
  id: string;
  name: string;
  duration: number;        // 时长（秒）
  type: 'once' | 'loop';   // 常规(执行一次停止) / 循环(到点自动重启)
  icon: string;            // emoji图标
  color: string;           // 主题色
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
}

export interface FixedReminder {
  id: string;
  name: string;            // 如"午饭"、"下班打卡"
  time: string;            // HH:mm 格式
  icon: string;            // emoji
  enabled: boolean;
  days?: number[];         // 周几生效（0=周日），undefined=每天
}

export interface TimerState {
  currentTaskId: string | null;
  isRunning: boolean;
  remainingSeconds: number;
  startedAt: number | null;  // 时间戳
  totalSessionsToday: number; // 今日完成的常规任务数
  totalFocusSecondsToday: number; // 今日专注总秒数
}
```

### 3.2 TimerStore（新增 `stores/timerStore.ts`）

- CRUD 计时任务
- 开始/暂停/重置计时
- 计时器状态管理（使用 setInterval，注意页面不可见时的处理）
- 固定提醒的开关管理
- 到点系统通知（Tauri Notification API）
- 每日统计数据持久化

### 3.3 Task 模型改动

优先级语义调整：保持数据库字段 `1|2|3` 不变，但UI展示改为：
- `1` = 低优先级（无标识/灰色）
- `2` = 中优先级（橙色圆点）
- `3` = 高优先级（红色圆点）
- 左侧色条：高=红、中=橙、低=不显示

> 注意：原代码中 priority=1 实际是高优先级（星星最多），需要确认是否需要迁移数据。
> 查看现有代码后决定：保持数据层不变，仅在UI展示层做映射调整。

---

## 四、UI 样式规范

### 4.1 设计Token（CSS变量）

```css
:root {
  --bg: #F5F5F0;
  --card-bg: #FFFFFF;
  --card-hover: #FAFAFA;
  --text-primary: #1A1A1A;
  --text-secondary: #888888;
  --text-muted: #AAAAAA;
  --accent: #4A90D9;
  --accent-light: rgba(74,144,217,0.1);
  --accent-hover: #5BA4F5;
  --danger: #E05252;
  --success: #28C840;
  --warning: #FFB800;
  --border: #E8E8E8;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-full: 999px;
  --p-high: #E05252;
  --p-medium: #FF9800;
  --timer-active: #FF6B6B;
}
```

### 4.2 侧边栏规范

- 宽度：60px
- 按钮：40x40px，圆角8px
- 三段式分组，分隔线宽28px、高1px
- 窗口控制按钮：11px 圆点
- active状态：蓝色背景+蓝色图标

### 4.3 任务卡规范

- 圆角12px，padding 12px 14px
- 左侧优先级色条3px宽（高=红、中=橙、低=无）
- 复选框：20px圆形
- 标签：11px字号，圆角胶囊
- hover：边框变蓝+阴影加深，不用scale
- 展开箭头在右侧，展开时旋转180度
- 操作按钮hover时显示（计时、编辑、删除）

### 4.4 计时器表盘

- SVG 圆环：r=90，stroke-width=8，圆角线帽
- 中心时间：42px JetBrains Mono 字体
- 开始按钮：红色圆形胶囊
- 暂停按钮：橙色圆形胶囊
- 运行中圆环变红色，状态文字红色

---

## 五、开发阶段

### 阶段1：布局重构（影响全局）
- [ ] 重写 Home.vue 布局：侧边栏三段式、统一窗口控制
- [ ] 重构 main.css 全局样式，使用CSS变量
- [ ] 适配Mac/Windows窗口控制统一到侧边栏顶部
- [ ] 状态栏底部展示
- [ ] 添加FAB浮动按钮

### 阶段2：任务卡重写
- [ ] 重写 TaskCard.vue 组件
- [ ] 适配 CategoryTabs 新样式（胶囊pill）
- [ ] 适配 SearchBar 新样式
- [ ] 优先级显示方式调整（色条+圆点）
- [ ] 任务卡展开/折叠（描述+缩略图）
- [ ] 附件数量指示器

### 阶段3：计时器功能
- [ ] 新增类型定义（TimerTask、FixedReminder、TimerState）
- [ ] 创建 timerStore.ts
- [ ] 实现 TimerClock.vue（SVG表盘+倒计时）
- [ ] 实现 TimerQuickStart.vue（快速启动卡片）
- [ ] 实现 TimerTaskList.vue（计时任务列表）
- [ ] 实现 FixedReminders.vue（固定提醒）
- [ ] 实现 TimerPanel.vue（面板整合）
- [ ] 集成到Home.vue侧边栏导航
- [ ] 到点系统通知
- [ ] 计时中小红点指示

### 阶段4：收尾
- [ ] 精简模式适配
- [ ] 深色模式适配
- [ ] 剪贴板面板样式适配
- [ ] 测试验证所有功能
- [ ] 清理无用代码

---

## 六、注意事项

1. **不改动剪贴板功能**：剪贴板卡片逻辑复杂，这次只做样式适配，不改功能
2. **数据兼容**：不做数据库迁移，Task模型字段保持不变
3. **Naive UI 依赖**：现有弹窗、消息提示、下拉菜单仍用 Naive UI，新组件尽量不用Naive UI以保持轻量
4. **计时器精度**：使用 `Date.now()` 计算剩余时间，不依赖 setInterval 的精确性（标签页不可见时setInterval会被节流）
5. **通知权限**：首次使用计时器需请求通知权限
6. **原型文件**：`ui-prototype.html` 和 `ui-prototype-v2.html` 是原型参考文件，不提交到代码中（放在项目根目录供参考，可加入.gitignore）
