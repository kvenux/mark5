# Changelog

All notable changes to **Mark 5** will be documented in this file.

## [0.2.0] - 2026-11-26

首个可日常主力写作的公开 Beta 里程碑：核心编辑交互收敛、视觉与暗色主题全面打磨，并建立独立真机 GUI 视频回归体系。

### Added
- **真机 GUI E2E 规范与独立视频验证流水线** (`tests/gui/`)：支持 `pnpm test:gui` 与 `pnpm test:gui:build`，覆盖工作区、欢迎页、文件 CRUD、编辑器/标签/查找、全套行内格式与批注闭环、会话 CRUD、三栏协作、主题/设置/拖拽调宽、导出与历史、本地图片与空态防护等 9 组套件，强制全程录制 `.mp4` 并输出 `report.md` / `report.json`。
- **文档查找栏 (`Ctrl+F`)**：支持实时匹配计数同步、回车/方向切换、`Esc` 关闭，且匹配高亮实现零排版位移。
- **多标签快捷体验**：支持鼠标中键关闭标签、`Ctrl+W` 关闭、`Ctrl+Tab` / `Ctrl+Shift+Tab` 切换标签与每标签独立滚动位置记忆。
- **窗口级防丢稿保护**：窗口失焦（`blur`）与关闭前（`beforeunload`）自动冲刷待落盘的防抖保存。

### Fixed
- **数据安全与撤销栈**：修复跨标签防抖保存串写风险；保留跨标签与 Agent 改文后的独立撤销栈，并为 `docHistoryCache` / `docScrollMap` 增加 30 条 LRU 上限；修复文件重命名时的保存冲刷与缓存迁移；修复快速切换文档时的异步竞态。
- **快捷键与中文输入法兼容**：兼容 Windows 中文输入法 `Process` 按键回退至 `e.code`；在弹窗打开、文档面板隐藏或空态下拦截 `Ctrl+Z/Y/S/W/F` 防止穿透底层编辑器；支持 `Esc` 关闭版本历史与长粘贴预览弹窗。
- **查找高亮零位移**：覆盖 Muya 默认 `.mu-highlight` / `.mu-selection` 的 `inline-block` 与横向 padding，彻底消除 `Ctrl+F` 查找时字符变宽跳动，并修复查找导航按钮图标被全局按钮 padding 压缩的问题。
- **选区格式化与大纲跳转**：选区浮条统一调用 `muyaInstance.format(type)` 确保全部行内格式稳定生效；修复含行内代码/加粗的大纲标题跳转匹配。
- **Agent 协作与 Diff 审阅**：Diff 审阅卡与批注卡改为基于文本内容锚定；修复审阅卡首帧位置跳变与窄三栏标题挤压；修复流式改文未闭合行内语法与块引用光标换行；浮标贴边停靠不遮挡上文；新改文动画启动时自动收起旧 Diff 卡；三栏 0 标签空态下保留顶栏关闭按钮 `×`。
- **深色主题与视觉细节**：优化深色模式下 Prism 代码与 YAML Front-matter 语法高亮对比度；修复深色模式 Toast 浅底白字不可见问题；修复底栏模型芯片截断、标签溢出滚动条、`PASTED` 卡片第 4 行半裁切与设置页文本框拖拽角标。
- **导出稳健性**：为导出与复制富文本增加异常捕获、Toast 反馈与纯文本兜底。

## [0.1.0] - Initial Prototype
- Tauri v2 + Rust + Vue 3 + Muya 本地 Markdown 协同写作原型。
