# Mark 5

**Mark 5** 是一款基于 **Tauri v2 + Rust + Vue 3 + Muya** 构建的极轻量、AI-Native 原生 Markdown 桌面编辑器——**6.96 MB 单文件体积、多开仅 +40 MB、后台常驻 80 MB**，兼顾 Typora 级暖白排版与无头 Coding Agent 协同写作。

## 极致性能（同文档实测对比）

> 测试环境：Windows 11，压测文档含 3 张 Mermaid 图表 + 40 张大表 + 40 段代码块；内存口径为整棵进程树真实独占物理内存（`WorkingSetPrivate`）。

| 指标 | **Mark 5**（Tauri v2） | **MarkText 0.24**（Electron） | **VS Code 1.145**（Electron） |
| --- | :---: | :---: | :---: |
| **分发体积（单版本）** | **6.96 MB**（1 文件） | 382.9 MB（55×） | 1,006.6 MB（145×） |
| **单窗口内存（活跃）** | **156.5 MB** | 338.0 MB（-54%） | 381.2 MB（-59%） |
| **双窗口内存（活跃）** | **197.0 MB** | 545.4 MB（-64%） | 1,375.2 MB（-86%） |
| **第 2 窗口纯增量** | **+40.5 MB** | +207.4 MB | +994.0 MB |
| **双窗口后台/最小化** | **80.3 MB**（自动回收） | 287.8 MB | — |
| **双窗口稳态空载 CPU** | **0.74%** | 1.89% | 5.40% |

## 核心特性
- **极致低内存 & 原生多开**：Rust 原生后端（~6.5 MB）+ 共享 WebView2 单渲染进程 + 失焦/最小化自动触发 `LOW` 内存模式；支持应用内 `新建窗口 (Ctrl+Shift+N)` 低开销多开，也支持多进程独立并行启动（`mark5-doc-<pid>` 本地 MCP 端口隔离）。
- **三段式极简视图**：`文件`（类 IDE 全量文件树 + 代码预览）| `文档`（Markdown 列表 + 可点击跳转大纲）| `对话`（Claude 风格会话管理 + 三栏 AI 协作）。
- **Claude Docs 风格交互**：选区上浮胶囊工具条 + 右侧栏批注卡片、输入框顶部独立引用卡片、Agent 悬浮光标逐段修改动画 + 词级 Diff 审阅（逐段接受/退回）。

## 开发与构建
```bash
pnpm install
pnpm run typecheck
pnpm run dev        # 启动 Tauri v2 开发窗口
pnpm run build      # 编译 Release 版本（packages/desktop/src-tauri/target/release/mark5.exe）
node scripts/mem-profile.mjs  # 运行可复现内存基准压测
```
