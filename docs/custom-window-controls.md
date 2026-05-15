# 自定义窗口控制按钮方案

> 创建日期：2026-05-15 | 版本：v1.13.0（计划中）

## 背景

原布局使用 `titleBarStyle: overlay`，Mac 端依赖系统红黄绿按钮，与微信的自定义风格不一致。
用户希望 Mac 和 Windows 两端都使用自定义窗口控制按钮，保持视觉统一。

## 效果目标

- **Mac 端**：左上角三个自定义圆点按钮（红/黄/绿），hover 时显示 × / − / + 符号，风格与 macOS 原生一致
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

- header 左侧添加 Mac 端三个圆点按钮（`mac-window-controls`）
- header 右侧保留 Windows 端按钮（`win-controls`）
- 整个 header 区域绑定 `@mousedown="startWindowDrag"` 实现窗口拖拽
- 按钮区域排除拖拽（点击按钮不触发拖拽）

#### 3. `Home.vue` — 样式

- `.app-layout` 添加 `border-radius: 10px`（窗口圆角）
- `.global-header` 添加 `-webkit-app-region: drag`（整体可拖拽）
- `.global-header .header` 添加 `-webkit-app-region: no-drag`（按钮不可拖拽）
- Mac 红黄绿按钮：
  - 12×12px 圆点，gap 8px
  - 颜色：`#FF5F57`（红）、`#FFBD2E`（黄）、`#28C840`（绿）
  - hover 时显示符号（× / − / +），默认隐藏
- 移除旧的 `platform-mac` overlay 相关样式

#### 4. `Home.vue` — 逻辑

- `startWindowDrag(e)`：全局拖拽函数
  - 排除所有 `button` 和窗口控制区域
  - 调用 `appWindow.startDragging()`
- `hideToTray()`：Mac 红按钮点击 → 隐藏到托盘（非关闭）

### 功能保留

- 窗口拖拽（header 空白区域）
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
2. 确认 Mac 端三个圆点按钮显示正常
3. 确认 hover 时显示对应符号（× / − / +）
4. 确认点击各按钮功能正常（隐藏/最小化/最大化）
5. 确认 Mac 端窗口拖拽正常（点击 header 空白区域）
6. 确认 Windows 端窗口控制按钮正常工作
7. 确认窗口圆角显示正常
8. 确认精简模式（置顶小窗口）下布局正常
