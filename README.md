# MMY TodoList

简洁的跨平台任务管理桌面应用。MMY-Tools 系列项目之一。

![Logo](desktop/src-tauri/icons/logo.png)

## 功能简介

- 多分类任务管理（最多 9 个分类）
- 任务支持标题、描述、日期、优先级、图片
- 拖拽排序、实时搜索
- 主题切换（亮色/暗色/跟随系统）
- 数据导出/导入
- 系统托盘（关闭窗口后应用继续运行）

## 运行开发

```bash
cd desktop
npm install
npm run tauri dev
```

## 打包发布

**一键打包脚本（项目根目录）：**

```bash
./build.sh        # 自动检测当前系统
./build.sh mac    # macOS DMG
./build.sh win    # Windows 便携版 EXE
```

**或在 desktop 目录下：**

```bash
npm run build:mac   # macOS DMG
npm run build:win   # Windows 便携版 EXE
```

**构建产物位置：**
- macOS: `desktop/src-tauri/target/release/bundle/dmg/`
- Windows: `desktop/src-tauri/target/release/bundle/nsis/`

> 注意：macOS DMG 只能在 macOS 上构建，Windows EXE 只能在 Windows 上构建。

## 使用教程

### 基本操作

1. **添加任务**：点击底部「添加任务」按钮
2. **编辑任务**：点击任务卡片右上角编辑图标
3. **删除任务**：点击任务卡片右上角删除图标
4. **切换状态**：点击任务卡片左侧圆点（待办 → 进行中 → 已完成）
5. **置顶任务**：点击任务卡片，在弹出的编辑框中设置置顶

### 分类管理

- 点击顶部文件夹图标进入分类管理
- 支持添加、编辑、删除、拖拽排序分类
- 最多支持 9 个分类

### 设置

- 点击顶部齿轮图标进入设置
- 主题切换、隐藏已完成任务、开机自启动

### 数据管理

- **导出**：设置页面 → 导出数据 → 保存 `.mmytodo` 文件
- **导入**：设置页面 → 导入数据 → 选择 `.mmytodo` 文件
- **数据位置**：
  - macOS: `~/Library/Application Support/com.mmy-tools.todolist/`
  - Windows: `%APPDATA%\com.mmy-tools.todolist\`

## 版本更新

### v1.0.0
- 初始版本发布
- 核心任务管理功能
- 分类管理（最多 9 个）
- 拖拽排序
- 主题切换
- 数据导出/导入
- 系统托盘支持

---

**技术栈**：Tauri 2 + Vue 3 + TypeScript + Naive UI + SQLite