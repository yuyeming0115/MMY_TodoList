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

- 多分类任务管理（支持拖拽排序、颜色标识）
- 拖拽排序、实时搜索
- 亮色/暗色/跟随系统主题
- 系统托盘后台运行
- 缩略图预览与编辑（支持缩放/旋转/翻转）
- 数据导出/导入（`.mmytodo` 格式）
- 字体大小与字体自定义
- 精简模式（紧凑显示任务卡片）
- 右键任务卡快速置顶
- 窗口置顶与窗口尺寸/位置记忆
- 剪贴板管理（自动监控+手动保存，支持文本和图像分类）

详见 [desktop/README.md](desktop/README.md) 和 [开发文档/开发说明.md](开发文档/开发说明.md)
