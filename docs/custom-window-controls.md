# 自定义窗口控制按钮方案

> 创建日期：2026-05-15 | 版本：v1.13.0

## 背景

原布局使用 `titleBarStyle: overlay`，Mac 端依赖系统红黄绿按钮，与微信的自定义风格不一致。
用户希望 Mac 和 Windows 两端都使用自定义窗口控制按钮，保持视觉统一。

最终效果：Mac 端红黄绿按钮融入左侧导航栏顶部，与微信 Mac 版布局一致。

## 效果目标

- **Mac 端**：左侧栏顶部三个自定义圆点按钮（红/黄/绿），hover 时显示 × / − / + 符号，风格与 macOS 原生一致
- **Windows 端**：右上角自定义按钮（最小化/最大化/关闭），风格与微信 Win 版一致
- **共同**：无系统标题栏，窗口完全自定义绘制，支持拖拽移动

## 实现方案

### 核心思路

将 `decorations` 改为 `false`，完全隐藏系统标题栏，用 CSS 绘制所有窗口控制元素。

### 涉及文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `desktop/src-tauri/tauri.conf.json` | 修改 | `decorations: false`，移除 `titleBarStyle` |
| `desktop/src/views/Home.vue` | 修改 | 模板、样式、逻辑 |

### 详细变更

#### 1. `tauri.conf.json`

```json
"decorations": false,  // 隐藏系统标题栏
"transparent": true,   // 保持透明，支持圆角
// 移除 "titleBarStyle": "overlay"
```

#### 2. `Home.vue` — 模板

- **Mac 端**：将 `mac-window-controls` 放在 `.sidebar` 内部（导航按钮上方），与侧边栏融为一体
- **Mac 端**：移除 `.global-header` 区域，内容区直接从窗口顶部开始
- **Windows 端**：保留 `.global-header`，按钮在右上角
- 侧边栏绑定 `@mousedown="startWindowDrag"` 实现窗口拖拽
- 按钮区域通过 `@click.stop` 和 CSS `no-drag` 排除拖拽

#### 3. `Home.vue` — 样式

- `.app-layout` 添加 `border-radius: 10px`（窗口圆角）
- `.sidebar` 添加 `-webkit-app-region: drag`（侧边栏可拖拽）
- `.sidebar-buttons` 添加 `-webkit-app-region: no-drag`（按钮不可拖拽）
- `.mac-window-controls` 添加 `-webkit-app-region: no-drag`（红黄绿按钮不可拖拽）
- Mac 红黄绿按钮：
  - 12×12px 圆点，gap 8px，侧边栏内水平居中
  - 顶部 padding 8px，底部 6px
  - 颜色：`#FF5F57`（红）、`#FFBD2E`（黄）、`#28C840`（绿）
  - hover 时显示符号（× / − / +），默认隐藏
- `.panel-tabs` 的 `padding-top` 从 4px 改为 0，内容贴到窗口顶部
- 移除旧的 `platform-mac` overlay 相关样式

#### 4. `Home.vue` — 逻辑

- `startWindowDrag(e)`：全局拖拽函数
  - 排除所有 `button` 和窗口控制区域（`.mac-window-controls`、`.win-controls`）
  - 调用 `appWindow.startDragging()`
- `hideToTray()`：Mac 红按钮点击 → 隐藏到托盘（非关闭）

### 布局示意

```
Mac 端：
┌──────────────────────────────────────────
│ [红黄绿]  │
│  📌      │  [全部] [工作] [事业] [+]
│  📋      │  [+ 任务] [搜索框...] [视图]
│          │  ──────────────────────────
│  ☀️      │  任务列表内容区域
│  中      │
│  ⚙️      │
└──────────┴──────────────────────────────

Windows 端：
┌──────────────────────────────────────────
│                  [最小化][最大化][关闭]  │
├──────────┬───────────────────────────────┤
│  📌      │  [全部] [工作] [事业] [+]
│  📋      │  [+ 任务] [搜索框...] [视图]
│          │  ──────────────────────────
│  ☀️      │  任务列表内容区域
│  中      │
│  ⚙️      │
└────────────────────────────────────────
```

### 功能保留

- 窗口拖拽（Mac：侧边栏空白区域；Win：header 空白区域）
- 窗口最小化/最大化/还原
- 隐藏到托盘（红按钮）
- 窗口置顶 + 精简模式
- 分类拖拽排序
- 所有面板切换功能

### 技术要点

- `decorations: false` 在 Mac 上是常用做法，微信/钉钉等都用这种方式
- `transparent: true` 已配置，圆角不会有问题
- 窗口管理 API（minimize/maximize/close）在 Tauri v2 中都有对应 JS 方法
- 红按钮点击 `hideToTray()` 而非 `appWindow.close()`，符合应用行为（关闭即隐藏）

### 测试验证

1. 启动开发服务器：`npm run tauri dev`
2. 确认 Mac 端红黄绿按钮在侧边栏顶部，与导航栏融为一体
3. 确认 Mac 端内容区贴到窗口顶部，无多余空白
4. 确认 hover 时显示对应符号（× / − / +）
5. 确认点击各按钮功能正常（隐藏/最小化/最大化）
6. 确认 Mac 端窗口拖拽正常（点击侧边栏空白区域或内容区 tabs 空白区域）
7. 确认 Windows 端窗口控制按钮正常工作
8. 确认窗口圆角显示正常
9. 确认精简模式（置顶小窗口）下布局正常
