# MMY TodoList

简洁的任务管理应用，面向中国用户。MMY-Tools 系列项目之一。

## 项目结构

```
MMY_TodoList/
├── desktop/             # Tauri + Vue3 桌面端
│   ├── src/             # Vue3 前端代码
│   │   ├── components/ # UI 组件
│   │   ├── views/      # 页面视图
│   │   ├── stores/     # Pinia 状态管理
│   │   ├── utils/      # 工具函数
│   │   └── types/      # TypeScript 类型定义
│   └── src-tauri/      # Rust 后端代码
│       └── src/
│           ├── models.rs   # 数据模型
│           ├── database.rs # SQLite 操作
│           ├── commands.rs # Tauri 命令
│           └── lib.rs      # 入口
│
├── mobile/              # 微信小程序（待开发）
│
├── 参考模板/             # UI 参考模板
│
├── 开发文档/
│   ├── 产品设计文档.md
│   └── 重构方案.md
│
└── README.md
```

## 桌面端开发

```bash
cd desktop
npm install
npm run tauri dev    # 开发模式
npm run tauri build  # 构建发布
```

## 打包发布

使用项目根目录的一键打包脚本：

```bash
# macOS（构建 DMG 安装包）
./build.sh mac

# Windows（构建便携版 EXE）
./build.sh win

# 自动检测当前系统
./build.sh
```

或在 desktop 目录下使用 npm 命令：

```bash
cd desktop
npm run build:mac   # 构建 DMG
npm run build:win   # 构建便携版 EXE
```

**构建产物位置：**
- macOS: `desktop/src-tauri/target/release/bundle/dmg/`
- Windows: `desktop/src-tauri/target/release/bundle/nsis/`

**注意：** macOS DMG 只能在 macOS 上构建，Windows EXE 只能在 Windows 上构建（Tauri 不支持交叉编译）。

## 技术栈

**桌面端：**
- Tauri 2（Rust 后端）
- Vue3 + TypeScript
- Naive UI（UI 框架）
- Pinia（状态管理）
- SQLite（数据存储）

**移动端（规划）：**
- 微信小程序
- 微信云开发

## 功能特性

- 分类管理（最多 9 个分类）
- 任务卡片（标题、描述、日期、优先级、图片）
- 拖拽排序
- 实时搜索
- 主题切换（亮色/暗色/跟随系统）
- 数据导出/导入（.mmytodo 格式）
- macOS 风格窗口控制按钮
- 系统托盘（规划）
- 开机自启动（规划）

## 开发进度

- [x] 桌面端项目骨架
- [x] Rust 数据库模块（SQLite CRUD）
- [x] Vue3 前端基础结构
- [x] 分类/任务/设置 Pinia Store
- [x] 核心组件（TaskCard、CategoryTabs、SearchBar）
- [ ] 任务表单弹窗
- [ ] 分类管理弹窗
- [ ] 设置页面
- [ ] 拖拽排序功能
- [ ] 系统托盘
- [ ] 微信小程序