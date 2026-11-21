# Mark 5

**Mark 5** 是一款基于 **Tauri v2 + Rust + Vue 3 + Muya** 构建的极轻量、AI-Native 原生 Markdown 桌面编辑器（Typora 级暖白排版体验 + 无头 Coding Agent 协同写作 + 原生多开低内存）。

## 核心特性
- **极致低内存 & 原生多开**：Rust 原生后端 + 系统共享 WebView2；支持应用内 `新建窗口 (Ctrl+Shift+N)` 共享后端低开销多开，也支持多进程独立并行启动（每进程自动分配独立随机端口与 `mark5-doc-<pid>` 本地 MCP，互不冲突）。
- **三段式极简视图**：`文件`（类 IDE 全量文件树 + 代码预览）| `文档`（Markdown 列表 + 可点击跳转大纲）| `对话`（Claude 风格会话管理 + 三栏 AI 协作）。
- **Claude Docs 风格交互**：选区上浮胶囊工具条 + 深色快捷键 Hint（`批注 Ctrl+Alt+M` / `引用到对话 Ctrl+Shift+L` / 8 种行内格式）、输入框顶部独立引用卡片、Agent 悬浮光标逐段修改动画、Markdown 富文本回复与代码块一键复制。

## 开发与构建
```bash
pnpm install
pnpm run typecheck
pnpm run dev        # 启动 Tauri v2 开发窗口
pnpm run build      # 编译 Release 版本（packages/desktop/src-tauri/target/release/mark5.exe）
```
