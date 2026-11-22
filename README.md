<div align="center">

# Mark 5

**7 MB 极轻体积，Claude Docs 式的 AI 原生 Markdown 编辑器。**

Typora 级暖白排版 + 划词批注与 AI 原地改稿，双开复杂长文不到 200 MB，挂在后台只占 80 MB。

![体积](https://img.shields.io/badge/体积-7%20MB-C2532B?style=flat-square)
![双开内存](https://img.shields.io/badge/双开内存-197%20MB-15803D?style=flat-square)
![后台常驻](https://img.shields.io/badge/后台常驻-80%20MB-15803D?style=flat-square)
![AI 协同](https://img.shields.io/badge/AI%20协同-Claude%20%7C%20Codex%20%7C%20AGY-1C1917?style=flat-square)

<br />

<img src="docs/assets/preview-comment.png" alt="Mark 5 暖白排版与 Claude Docs 式划词批注" width="900" />

</div>

## 极致轻量：双开比别人单开还省

<p align="center">
  <img src="docs/assets/benchmark.svg" alt="Mark 5 vs MarkText vs VS Code 体积与双开内存对比" width="880" />
</p>

<p align="center">
  多开一窗仅增 <strong>+40 MB</strong>，最小化 20 秒自动降至 <strong>80 MB</strong> · <a href="docs/BENCHMARKS.md">查看完整实测数据与复现脚本 →</a>
</p>

## 核心体验

- **Claude Docs 式协同写作**：选中任意段落即可发起侧栏批注或引用到对话，AI 光标逐段原地改写，支持词级 Diff 逐段采纳或一键退回。
- **极致轻量，随手多开**：7 MB 原生体积，双开窗口不到 200 MB、待机 CPU 不足 1%，`Ctrl+Shift+N` 随时多开不心疼，最小化自动释放内存。
- **Typora 级沉浸排版**：暖白纸面与所见即所得编辑，大纲一键跳转，原生支持 Mermaid 图表、表格与多语言代码高亮。
- **开箱直连本机 Agent**：自动发现并接入本机已安装的 Claude Code / Codex / AGY，无需重复配置，直接读写你的本地 Markdown 文档。

## 快速开始

```bash
pnpm install
pnpm run dev                  # 启动开发窗口
pnpm run build                # 构建 Release（packages/desktop/src-tauri/target/release/mark5.exe）
node scripts/mem-profile.mjs  # 一键复测内存基准
```

<sub>基于 Tauri v2 + Rust + Vue 3 + Muya 构建。</sub>
