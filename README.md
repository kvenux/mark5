# Mark 5

**7 MB 的 AI 原生 Markdown 编辑器。**

Typora 级流畅写作体验，内置 Claude / Codex / AGY 协同改稿与划词批注——开两个窗口不到 200 MB，挂在后台只占 80 MB。

## 极致轻量：双开比别人单开还省

| | **Mark 5** | MarkText | VS Code |
| --- | :---: | :---: | :---: |
| **软件大小** | **7 MB** | 383 MB | 1.0 GB |
| **单开内存** | **156 MB** | 338 MB | 381 MB |
| **双开内存** | **197 MB** | 545 MB | 1,375 MB |
| **多开一窗** | **+40 MB** | +207 MB | +994 MB |
| **后台常驻** | **80 MB** | 288 MB | — |
| **双开待机 CPU** | **< 1%** | 1.9% | 5.4% |

<sub>Windows 11 同文档实测（含 3 张 Mermaid 图表、40 张表格与 40 段代码块，统计整棵进程树真实物理内存）。</sub>

## 核心能力
- **沉浸写作**：所见即所得排版，大纲一键跳转，完整支持 Mermaid 图表、表格与代码高亮。
- **AI 原地协同**：划词发起批注或引用到对话，Agent 逐段修改文档，支持词级 Diff 逐段接受或退回。
- **随手多开**：`Ctrl+Shift+N` 秒开新窗口，内存几乎无感；最小化 20 秒自动释放内存，切回即用。

## 开发与构建
基于 Tauri v2 + Rust + Vue 3 + Muya 构建：

```bash
pnpm install
pnpm run typecheck
pnpm run dev                  # 启动开发窗口
pnpm run build                # 构建 Release（packages/desktop/src-tauri/target/release/mark5.exe）
node scripts/mem-profile.mjs  # 复测内存基准
```
