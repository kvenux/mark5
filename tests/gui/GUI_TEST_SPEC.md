# Mark 5 Real-Binary GUI E2E Test Specification

> **强制要求**：所有 GUI 测试套件必须在**真实编译的 `mark5.exe`（Tauri v2 WebView2 + Rust 后端 + 真实磁盘）**上运行，并**强制全程录制 10 FPS `.mp4` 视频**。单次验收仅在满足以下全部条件时通过：
> 1. 所有功能断言（`assert`）100% 通过；
> 2. WebView2 控制台 `console.error` 与未捕获异常（`Runtime.exceptionThrown`）为 `0`；
> 3. 每个套件均产出有效 `.mp4` 视频（非 0 字节），且完成人工或视觉模型逐帧核验无渲染错位、遮挡、文字截断或跳变。

---

## 1. 运行方式

```bash
# 1. 对当前已编译的 release mark5.exe 运行全量 9 组真机 GUI 测试 + 录屏
pnpm test:gui

# 2. 先编译最新 mark5.exe，再运行全量真机 GUI 测试 + 录屏
pnpm test:gui:build

# 3. 仅运行指定套件（逗号分隔）
pnpm test:gui -- --suite S2,S4

# 4. 指定产物输出目录（默认输出至 tests/gui/artifacts/<timestamp>/）
pnpm test:gui -- --out C:/path/to/output
```

### 环境前置条件
- Windows + WebView2 Runtime（支持 `--remote-debugging-port`）
- Node.js 22+（内置 `WebSocket` 与 `fetch`）
- `ffmpeg` 在 `PATH` 中可用（用于将连续截图帧封装为 `yuv420p` H.264 `.mp4`）
- 已执行 `pnpm build` 产出 `packages/desktop/src-tauri/target/release/mark5.exe`（或运行 `pnpm test:gui:build`）

---

## 2. 产物结构

每次运行会在输出目录（默认 `tests/gui/artifacts/<run-id>/`）生成：
- `S1-workspace-welcome.mp4` ~ `S9-images-empty-guards.mp4`：各套件全程操作录屏（即使断言失败也会生成现场取证视频）
- `report.json`：机器可读的套件状态、耗时、断言详情、错误列表、视频路径与文件大小
- `report.md`：可读验收报告，包含可点击的 `file://` 视频链接与逐帧视觉核验清单

---

## 3. 全量测试套件矩阵（S1 – S9）

### S1 — 工作区、欢迎页与侧栏导航 (`S1-workspace-welcome.mp4`)
- **覆盖模块**：`LeftSidebar.vue`、`WelcomeView.vue`、`FileTreeNode.vue`
- **操作步骤**：
  1. 打开底部工作区切换弹层并关闭。
  2. 切换至「文件」模式，展开 `src` 目录并点击 `config.ts` 验证代码只读预览。
  3. 切换回「文档」模式，激活 `posts/rust-async.md`，点击大纲中含行内代码的 `1. tokio::spawn 任务队列` 标题验证跳转。
  4. 收起并重新展开侧栏（专注模式切换）。
  5. 点击左上角 `Mark 5` 品牌返回欢迎页 `WelcomeView`，从欢迎页打开设置弹窗再 `Esc` 关闭，点击「最近打开」列表重新进入工作区。
- **视频核验要点**：欢迎页排版居中、最近列表路径清晰、大纲跳转准确、侧栏折叠无残影。

### S2 — 文件与目录完整 CRUD (`S2-file-crud.mp4`)
- **覆盖模块**：`LeftSidebar.vue`、`FileTreeNode.vue`、`appStore.ts`
- **操作步骤**：
  1. 点击侧栏「新建文档」，验证自动创建新 `.md` 文件并打开。
  2. 切换至「文件」模式，右键 `posts` 目录 -> 「新建子文件夹」（通过真实 CDP Dialog 输入 `sub-specs`），验证目录树出现 `sub-specs`。
  3. 右键 `notes.md` -> 「创建副本」，验证副本文件出现。
  4. 右键副本文件 -> 「重命名」，内联输入 `renamed-note.md` 并回车，验证磁盘与文件树同步更新。
  5. 右键 `renamed-note.md` -> 「删除」（通过真实 CDP Confirm 确认），验证文件从树中与磁盘移除。
- **视频核验要点**：右键菜单定位不越界、内联重命名框清晰、删除后树刷新流畅。

### S3 — 编辑器核心、多标签、滚动记忆、查找与保存 (`S3-editor-tabs-find.mp4`)
- **覆盖模块**：`MuyaEditorPane.vue`、`App.vue`
- **操作步骤**：
  1. 打开 `posts/rust-async.md` 与 `notes.md` 形成多标签。
  2. 滚动 `rust-async.md` 至 `scrollTop = 420`，用 `Ctrl+Tab` 切到 `notes.md`，再用 `Ctrl+Shift+Tab` 切回，断言滚动位置恢复。
  3. 在正文末尾输入文字，`Ctrl+S` 真实落盘，断言磁盘内容更新且 Toast 出现。
  4. 执行 `Ctrl+Z` 撤销与 `Ctrl+Y` 重做，断言正文状态准确变化。
  5. `Ctrl+F` 打开查找栏输入 `异步`，断言**高亮行高与字宽变化 ≤ 0.5px（零位移）**；按 `Enter` 切换命中项；`Esc` 关闭查找栏。
  6. 使用**鼠标中键 (`auxclick`)** 关闭 `notes.md` 标签，断言标签数量减少。
- **视频核验要点**：`Ctrl+F` 高亮不撑宽字距、查找栏图标居中清晰、Toast 对比度可读。

### S4 — 选区浮条全量行内格式与批注完整闭环 (`S4-selection-comments.mp4`)
- **覆盖模块**：`MuyaEditorPane.vue`
- **操作步骤**：
  1. 依次选中段落词组并点击选区浮条全部行内格式按钮：**加粗、斜体、下划线、删除线、行内代码、高亮、清除格式**，断言对应 DOM 标签生成与清除。
  2. 选中词组点击「批注」，输入批注内容并点击「发送」，断言右侧边栏批注卡渲染。
  3. 点击批注卡展开回复框，输入回复内容并点击「发送」，断言回复消息数增加。
  4. 点击批注卡「解决」，断言批注从待处理列表移除。
- **视频核验要点**：选区浮条在选区正上方不遮挡文字、批注卡锚定于右侧边栏不遮挡正文、回复与解决过渡自然。

### S5 — 对话会话完整 CRUD (`S5-sessions-crud.mp4`)
- **覆盖模块**：`LeftSidebar.vue`、`appStore.ts`
- **操作步骤**：
  1. 切换至「对话」模式，点击「新建会话」创建 2 个会话。
  2. 打开会话 `···` 菜单 -> 「置顶」，断言 `Pinned` 分组出现。
  3. 打开菜单 -> 「重命名」，内联输入 `架构评审会话` 并回车，断言标题更新。
  4. 打开菜单 -> 「归档」，断言 `已归档` 折叠分组出现并点击展开。
  5. 打开归档会话菜单 -> 「删除」，断言会话移除。
  6. 点击侧栏「搜索」输入关键词，验证会话实时过滤。
- **视频核验要点**：会话分组折叠/展开整齐、菜单位置正确、内联重命名不溢出。

### S6 — 三栏对话协作、引用芯片、PASTED 卡片与引擎/模型选择 (`S6-chat-collab.mp4`)
- **覆盖模块**：`ChatPane.vue`、`MuyaEditorPane.vue`
- **操作步骤**：
  1. 编辑器选区点击「引用到对话」，断言引用芯片出现在输入区，点击「清空」移除引用后再重新引用一次。
  2. 在对话输入框粘贴超长文本，断言生成 3 行截断的 `PASTED` 卡片。
  3. 点击 `PASTED` 卡片打开预览弹窗，编辑内容后点击「保存修改」，再次打开按 `Esc` 关闭，随后点击卡片 `×` 删除。
  4. 打开底栏模型下拉弹窗，校验「默认」模型芯片无截断，选择模型后关闭；切换引擎下拉框（AGY / Claude Code / Codex）。
  5. 点击文档区顶栏 `×` 关闭文档面板，再点击对话栏「打开文档」重新打开。
- **视频核验要点**：底栏无文字截断、`PASTED` 卡片无半行裁切、三栏/两栏切换平滑。

### S7 — 主题、设置项生效与分栏拖拽调宽 (`S7-themes-settings-layout.mp4`)
- **覆盖模块**：`SettingsModal.vue`、`App.vue`
- **操作步骤**：
  1. `Ctrl+,` 打开设置弹窗，依次切换三套主题（暖纸 / 纯净白 / 深邃暗）。
  2. 切换正文字号（`14px` -> `17px` -> `15px`）与纸面最大宽度（`680px` -> `960px` -> `720px`），断言 CSS 变量与样式生效。
  3. 切至「AI 后端与模型」Tab，点击「检测本地 CLI」，验证按钮响应无报错。
  4. 切至「个人写作偏好」Tab，点击启停开关、编辑文本并点击「恢复默认」，`Esc` 关闭弹窗。
  5. 拖拽侧栏分隔条调整宽度，并切换到「对话」模式拖拽对话/文档分隔条，断言宽度实际变化。
- **视频核验要点**：深色模式 YAML 与 Rust 代码高亮高对比度、设置弹窗滚动条与 textarea 无拖拽角标、分栏拖拽顺滑无卡顿。

### S8 — 文档操作、复制富文本与版本历史弹窗 (`S8-export-history-rollback.mp4`)
- **覆盖模块**：`MuyaEditorPane.vue`、`App.vue`
- **操作步骤**：
  1. 编辑文档并连续触发保存，确保落盘成功。
  2. 打开「文档操作」菜单，点击「复制富文本」，断言出现反馈 Toast。
  3. 打开「文档操作」菜单 -> 「版本历史」，验证弹窗正常展示；在弹窗打开期间按 `Ctrl+Z` 断言快捷键被拦截（不破坏底层编辑器）；若存在历史快照则执行回滚校验；`Esc` 关闭弹窗。
- **视频核验要点**：菜单对齐、深色/浅色下 Toast 可读、历史弹窗居中且 `Esc` 即刻关闭。

### S9 — 本地图片加载、三栏空态与快捷键防护 (`S9-images-empty-guards.mp4`)
- **覆盖模块**：`MuyaEditorPane.vue`、`App.vue`
- **操作步骤**：
  1. 滚动至嵌套相对路径本地图片 `![本地架构图](./arch.png)`，断言 `img.complete === true && img.naturalWidth > 0`。
  2. 切换至深色主题并进入「对话」三栏模式。
  3. 按 `Ctrl+W` 关闭所有标签，断言空态 `新建文档` 显示且**顶栏关闭文档面板 `×` 按钮保持可见**。
  4. 在空态下触发 `Ctrl+Z` / `Ctrl+S` / `Ctrl+F`，断言 0 异常、不意外弹出查找栏。
- **视频核验要点**：本地图片清晰加载、深色三栏空态顶栏 `×` 可见且对比度清晰。
