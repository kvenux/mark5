# Mark 5

**Mark 5** 是一款基于 **Tauri v2 + Rust + Vue 3 + Muya** 构建的极轻量、AI-Native 原生 Markdown 桌面编辑器——Typora 级暖白排版 + 无头 Coding Agent 协同写作，**轻到可以一直挂在后台、随便多开**。

## 为什么快、为什么轻？

> **不到 7 MB 单文件，双开复杂长文不到 200 MB，最小化挂后台仅占 80 MB**——告别 Electron 动辄几百 MB 的体积与内存焦虑，多开窗口几乎无感。

| 对比项 | **Mark 5** | MarkText | VS Code | 一句话感受 |
| --- | :---: | :---: | :---: | --- |
| **软件体积** | **6.96 MB** | 383 MB | 1.0 GB | 单文件即下即用，小 **55~145 倍** |
| **单开内存** | **156 MB** | 338 MB | 381 MB | 打开复杂图表长文，直接省一半以上 |
| **双开内存** | **197 MB** | 545 MB | 1,375 MB | 双开比别人**单开**还省 |
| **每多开 1 个窗口** | **+40 MB** | +207 MB | +994 MB | 共享引擎，随便多开不心疼 |
| **最小化挂后台** | **80 MB** | 288 MB | — | 切走 20 秒自动释放内存，切回秒醒 |
| **双开挂机 CPU** | **< 1%** | 1.9% | 5.4% | 长期常驻，风扇安静 |

<sub>同机 Windows 11 实测（复杂文档含 3 张 Mermaid 图表 + 40 张表格 + 40 段代码块，统计整棵进程树真实物理内存）。</sub>

## 核心特性
- **原生轻量 & 无感多开**：Rust 后端（~6.5 MB）+ 共享 WebView2 渲染进程；支持应用内 `新建窗口 (Ctrl+Shift+N)` 低开销多开，也支持多进程独立并行启动（自动分配随机本地 MCP 端口，互不冲突）。
- **三段式极简视图**：`文件`（全量文件树 + 代码预览）| `文档`（Markdown 列表 + 可点击大纲）| `对话`（Claude 风格会话管理 + 三栏 AI 协作）。
- **Claude Docs 风格 AI 协同**：选区上浮工具条 + 右侧栏批注卡片、输入框顶部独立引用卡片、Agent 逐段修改动画 + 词级 Diff 审阅（逐段接受/退回）。

## 开发与构建
```bash
pnpm install
pnpm run typecheck
pnpm run dev                  # 启动 Tauri v2 开发窗口
pnpm run build                # 编译 Release 版本（packages/desktop/src-tauri/target/release/mark5.exe）
node scripts/mem-profile.mjs  # 一键复测内存基准
```
