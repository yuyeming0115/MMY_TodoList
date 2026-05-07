# MMY TodoList

简洁的跨平台任务管理桌面应用。MMY-Tools 系列项目之一。

## 快速开始

```bash
cd desktop
npm install
npm run tauri dev
```

## 打包发布

```bash
./build.sh        # 自动检测当前系统
./build.sh mac    # macOS DMG
./build.sh win    # Windows 便携版 EXE
```

## 功能特性

- 多分类任务管理
- 拖拽排序、实时搜索
- 亮色/暗色主题
- 系统托盘后台运行
- 缩略图预览与编辑
- 数据导出/导入（`.mmytodo` 格式）

详见 [desktop/README.md](desktop/README.md)
