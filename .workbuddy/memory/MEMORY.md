# MMY TodoList - 工作记忆

## 项目概况
- **技术栈**: Vue 3 + Tauri v2 + Pinia + Naive UI + TypeScript + Rust/SQLite
- **项目路径**: `/Users/zoe/Desktop/GitWork/MMY_TodoList/desktop/`
- **构建**: Vite 6 + vue-tsc（需从 desktop/ 目录执行构建命令）

## 重要发现

### vuedraggable 拖拽排挤滑动修复 (2026-05-02)
- **根因1**: `.task-wrapper` 和 `.simple-card` 的 `transition: all` / `transition: transform` 会覆盖 SortableJS 设置的内联 transition，导致排挤滑动动画完全失效
- **根因2**: `handle=".drag-handle"` 限制只能通过小图标拖拽，体验差，已移除
- **根因3**: `dragEnabled` 逻辑中 `!settingsStore.settings.hideCompletedTasks` 在隐藏已完成任务时禁用拖拽，不合理，已修改为只在搜索时禁用
- **根因4**: Tauri WebView 中原生 HTML5 拖拽可能不兼容，添加 `forceFallback: true`
- **修复方案**: 
  - 移除 `.simple-card` 的 `transition: transform` 和 `:hover` 的 transform
  - `.task-wrapper` 不设置任何 transition
  - 添加 `forceFallback: true` 和 `fallbackTolerance: 3`
  - 移除 `handle` 属性让整个卡片可拖拽
- **归档文档**: 已创建 `/Users/zoe/Desktop/GitWork/MMY_TodoList/开发文档/拖拽排挤滑动修复.md`
