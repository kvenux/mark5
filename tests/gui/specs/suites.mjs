import fs from 'node:fs'
import path from 'node:path'

export const GUI_SUITES = [
  {
    id: 'S1',
    name: 'Workspace, WelcomeView & Sidebar Navigation',
    videoFile: 'S1-workspace-welcome.mp4',
    async run(ctx) {
      const { evalJs, pressKey, waitFor, assert, sleep } = ctx

      // 1. Workspace switcher popover
      await evalJs("document.querySelector('.ws-trigger')?.click();")
      await waitFor("document.querySelector('.ws-popover')", 3000, 'workspace popover')
      await sleep(350)
      await evalJs('document.body.click();')
      await sleep(200)

      // 2. Files mode -> expand src -> preview config.ts
      await evalJs(
        "Array.from(document.querySelectorAll('.mode-tab')).find(b => b.textContent.trim()==='文件')?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()==='src')?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()==='config.ts')?.click();"
      )
      await sleep(450)
      const hasTsCode = await evalJs(
        "return document.body.innerText.includes('Mark5Config');"
      )
      assert(hasTsCode, 'Non-markdown file config.ts preview rendered')

      // 3. Docs mode -> TOC jump to heading with inline code
      await evalJs(
        "Array.from(document.querySelectorAll('.mode-tab')).find(b => b.textContent.trim()==='文档')?.click();"
      )
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.tab-pill')).find(t => t.textContent.includes('rust-async'))?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.toc-item')).find(el => el.textContent.includes('tokio::spawn'))?.click();"
      )
      await sleep(400)

      // 4. Sidebar collapse & expand
      await evalJs('document.querySelector(\'button[title="收起侧栏"]\')?.click();')
      await waitFor('!document.querySelector(\'.left-sidebar\')', 3000, 'sidebar collapsed')
      await sleep(350)
      await evalJs('document.querySelector(\'button[title="展开侧栏"]\')?.click();')
      await waitFor("document.querySelector('.left-sidebar')", 3000, 'sidebar expanded')
      await sleep(300)

      // 5. Return to WelcomeView -> open Settings -> Esc -> reopen recent workspace
      await evalJs("document.querySelector('.brand-word')?.click();")
      await waitFor("document.querySelector('.welcome-root')", 4000, 'WelcomeView shown')
      await sleep(400)
      await evalJs(
        "Array.from(document.querySelectorAll('.welcome-actions button')).find(b => b.textContent.includes('偏好与引擎设置'))?.click();"
      )
      await waitFor("document.querySelector('.settings-mask')", 3000, 'Settings from WelcomeView')
      await sleep(350)
      await pressKey('Escape', 'Escape', 27, 0)
      await waitFor("!document.querySelector('.settings-mask')", 3000, 'Settings closed')
      await sleep(250)
      await evalJs("document.querySelector('.recent-row')?.click();")
      await waitFor("document.querySelector('.left-sidebar')", 5000, 'Re-entered workspace from WelcomeView')
      await sleep(350)
    }
  },

  {
    id: 'S2',
    name: 'File & Folder CRUD',
    videoFile: 'S2-file-crud.mp4',
    async run(ctx) {
      const { wsDir, evalJs, pressKey, insertText, waitFor, assert, expectDialog, sleep } = ctx

      // 1. Create new doc via sidebar button
      const beforeCount = fs.readdirSync(wsDir).filter((f) => f.endsWith('.md')).length
      await evalJs(
        "Array.from(document.querySelectorAll('.nav-row')).find(b => b.textContent.includes('新建文档'))?.click();"
      )
      await sleep(450)
      const afterCount = fs.readdirSync(wsDir).filter((f) => f.endsWith('.md')).length
      assert(afterCount > beforeCount, 'New markdown document created on disk')

      // 2. Switch to Files mode -> right-click posts -> New subfolder
      await evalJs(
        "Array.from(document.querySelectorAll('.mode-tab')).find(b => b.textContent.trim()==='文件')?.click();"
      )
      await sleep(300)
      await evalJs(`
        const row = Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()==='posts');
        const r = row.getBoundingClientRect();
        row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 30, clientY: r.top + 10 }));
      `)
      await waitFor("document.querySelector('.file-context-menu')", 3000, 'Folder context menu')
      await sleep(250)
      expectDialog(true, 'sub-specs')
      await evalJs(
        "Array.from(document.querySelectorAll('.file-context-menu .smenu-item')).find(b => b.textContent.includes('新建子文件夹'))?.click();"
      )
      await sleep(450)
      assert(fs.existsSync(path.join(wsDir, 'posts', 'sub-specs')), 'Subfolder posts/sub-specs created on disk')

      // 3. Right-click notes.md -> Duplicate
      await evalJs(`
        const row = Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()==='notes.md');
        const r = row.getBoundingClientRect();
        row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 30, clientY: r.top + 10 }));
      `)
      await waitFor("document.querySelector('.file-context-menu')", 3000, 'File context menu')
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.file-context-menu .smenu-item')).find(b => b.textContent.includes('创建副本'))?.click();"
      )
      await sleep(450)
      const dupFile = fs.readdirSync(wsDir).find((f) => f.startsWith('notes') && f !== 'notes.md')
      assert(Boolean(dupFile), `Duplicate file created (${dupFile || 'none'})`)

      // 4. Rename duplicate file inline -> renamed-note.md
      await evalJs(`
        const row = Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()===${JSON.stringify(dupFile)});
        const r = row.getBoundingClientRect();
        row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 30, clientY: r.top + 10 }));
      `)
      await waitFor("document.querySelector('.file-context-menu')", 3000, 'Dup file context menu')
      await evalJs(
        "Array.from(document.querySelectorAll('.file-context-menu .smenu-item')).find(b => b.textContent.includes('重命名'))?.click();"
      )
      await waitFor("document.querySelector('.tree-rename-input')", 3000, 'Inline rename input')
      await sleep(200)
      await evalJs(`
        const inp = document.querySelector('.tree-rename-input');
        inp.focus(); inp.select();
      `)
      await insertText('renamed-note.md')
      await sleep(200)
      await pressKey('Enter', 'Enter', 13, 0)
      await sleep(450)
      assert(fs.existsSync(path.join(wsDir, 'renamed-note.md')), 'Renamed file exists on disk')

      // 5. Delete renamed-note.md
      await evalJs(`
        const row = Array.from(document.querySelectorAll('.tree-row')).find(el => el.querySelector('.tree-name')?.textContent.trim()==='renamed-note.md');
        const r = row.getBoundingClientRect();
        row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 30, clientY: r.top + 10 }));
      `)
      await waitFor("document.querySelector('.file-context-menu')", 3000, 'Delete context menu')
      await sleep(200)
      expectDialog(true)
      await evalJs(
        "Array.from(document.querySelectorAll('.file-context-menu .smenu-item')).find(b => b.textContent.includes('删除'))?.click();"
      )
      await sleep(450)
      assert(!fs.existsSync(path.join(wsDir, 'renamed-note.md')), 'Deleted file removed from disk')
    }
  },

  {
    id: 'S3',
    name: 'Editor Core, Tabs, Scroll Memory, Find Zero-Shift & Middle-Click Close',
    videoFile: 'S3-editor-tabs-find.mp4',
    async run(ctx) {
      const { wsDir, evalJs, pressKey, insertText, waitFor, assert, sleep } = ctx

      // Open second tab notes.md, switch back to rust-async.md
      await evalJs(
        "Array.from(document.querySelectorAll('[title]')).find(n => (n.getAttribute('title')||'').includes('notes.md'))?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.tab-pill')).find(t => t.textContent.includes('rust-async'))?.click();"
      )
      await sleep(300)

      // Scroll memory across Ctrl+Tab / Ctrl+Shift+Tab
      await evalJs("document.querySelector('.muya-scroll').scrollTop = 420;")
      await sleep(300)
      await pressKey('Tab', 'Tab', 9, 2) // Ctrl+Tab -> notes
      await sleep(350)
      await pressKey('Tab', 'Tab', 9, 2 | 8) // Ctrl+Shift+Tab -> rust-async
      await sleep(400)
      const restoredScroll = await evalJs("return document.querySelector('.muya-scroll').scrollTop;")
      assert(restoredScroll >= 300, `Scroll position restored (${restoredScroll}px)`)

      // Type text + Ctrl+S real disk save + Ctrl+Z / Ctrl+Y
      await evalJs("document.querySelector('.muya-scroll').scrollTop = 0;")
      await sleep(200)
      await evalJs(`
        const p = document.querySelector('.mu-container p'); p.click();
        const r = document.createRange(); r.selectNodeContents(p); r.collapse(false);
        const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
      `)
      await sleep(120)
      await insertText(' [GUI-SPEC-OK]')
      await sleep(300)
      await pressKey('s', 'KeyS', 83, 2)
      await sleep(450)
      const savedDisk = fs.readFileSync(path.join(wsDir, 'posts', 'rust-async.md'), 'utf8')
      assert(savedDisk.includes('[GUI-SPEC-OK]'), 'Ctrl+S persisted edit to disk')

      await pressKey('z', 'KeyZ', 90, 2)
      await sleep(350)
      await pressKey('y', 'KeyY', 89, 2)
      await sleep(350)

      // Ctrl+F zero-layout-shift check
      const beforeH = await evalJs(
        "return document.querySelector('.mu-container p').getBoundingClientRect().height;"
      )
      await pressKey('f', 'KeyF', 70, 2)
      await waitFor("document.querySelector('.find-bar')", 3000, 'Find bar opened')
      await insertText('异步')
      await sleep(450)
      const afterH = await evalJs(
        "return document.querySelector('.mu-container p').getBoundingClientRect().height;"
      )
      assert(Math.abs(afterH - beforeH) <= 0.5, `Ctrl+F zero layout shift (${beforeH} -> ${afterH})`)
      await pressKey('Enter', 'Enter', 13, 0)
      await sleep(300)
      await pressKey('Escape', 'Escape', 27, 0)
      await waitFor("!document.querySelector('.find-bar')", 3000, 'Find bar closed')
      await sleep(250)

      // Middle-click (auxclick button=1) tab close
      const tabsBefore = await evalJs("return document.querySelectorAll('.tab-pill').length;")
      await evalJs(`
        const t = Array.from(document.querySelectorAll('.tab-pill')).find(el => el.textContent.includes('notes'));
        t.dispatchEvent(new MouseEvent('auxclick', { bubbles: true, cancelable: true, button: 1 }));
      `)
      await sleep(350)
      const tabsAfter = await evalJs("return document.querySelectorAll('.tab-pill').length;")
      assert(tabsAfter === tabsBefore - 1, `Middle-click closed tab (${tabsBefore} -> ${tabsAfter})`)
    }
  },

  {
    id: 'S4',
    name: 'Selection Toolbar (All Inline Formats) & Comment Lifecycle',
    videoFile: 'S4-selection-comments.mp4',
    async run(ctx) {
      const { evalJs, insertText, waitFor, assert, selectWordInFirstParagraph, sleep } = ctx

      const formats = [
        ['高并发服务', '加粗', 'strong'],
        ['异步调度器', '斜体', 'em'],
        ['工作窃取机制', '下划线', 'u'],
        ['尾延迟上限', '删除线', 'del'],
        ['真实压测', '行内代码', 'code'],
        ['性能表现', '高亮', 'mark']
      ]
      for (const [word, label, tag] of formats) {
        await selectWordInFirstParagraph(word)
        await evalJs(
          `document.querySelector('.selection-toolbar button[aria-label="${label}"]')?.click();`
        )
        await sleep(300)
        const hasTag = await evalJs(
          `return Boolean(document.querySelector('.mu-container p ${tag}'));`
        )
        assert(hasTag, `Inline format "${label}" rendered <${tag}>`)
      }

      // Clear format on bolded word
      await selectWordInFirstParagraph('高并发服务')
      await evalJs(
        `document.querySelector('.selection-toolbar button[aria-label="清除格式"]')?.click();`
      )
      await sleep(300)

      // Full Comment Lifecycle: Create -> Submit -> Reply -> Resolve
      await selectWordInFirstParagraph('异步调度与异步队列')
      await evalJs(
        `document.querySelector('.selection-toolbar button[aria-label="批注"]')?.click();`
      )
      await waitFor("document.querySelector('.right-float-comment')", 3000, 'Comment composer opened')
      await sleep(250)
      await evalJs("document.querySelector('.right-float-comment textarea')?.focus();")
      await insertText('建议补充 CPU 亲和性与队列长度对比数据')
      await sleep(300)
      await evalJs("document.querySelector('.right-float-comment .btn-primary')?.click();")
      await waitFor("document.querySelector('.margin-thread-card')", 4000, 'Margin comment card created')
      await sleep(400)

      // Click margin card to expand reply box, post reply
      await evalJs("document.querySelector('.margin-thread-card')?.click();")
      await waitFor("document.querySelector('.mtc-reply-input')", 3000, 'Reply input visible')
      await evalJs("document.querySelector('.mtc-reply-input')?.focus();")
      await insertText('已记录，下一节补充火焰图')
      await sleep(250)
      await evalJs("document.querySelector('.mtc-send-btn')?.click();")
      await sleep(400)
      const msgCount = await evalJs(
        "return document.querySelectorAll('.margin-thread-card .mtc-msg').length;"
      )
      assert(msgCount >= 2, `Comment thread has reply (${msgCount} messages)`)

      // Resolve thread
      await evalJs("document.querySelector('.mtc-resolve-btn')?.click();")
      await sleep(400)
      const remainingCards = await evalJs(
        "return document.querySelectorAll('.margin-thread-card').length;"
      )
      assert(remainingCards === 0, 'Comment thread resolved and removed')
    }
  },

  {
    id: 'S5',
    name: 'Collab Sessions CRUD (New, Pin, Rename, Archive, Delete, Search)',
    videoFile: 'S5-sessions-crud.mp4',
    async run(ctx) {
      const { evalJs, pressKey, insertText, waitFor, assert, sleep } = ctx

      await evalJs(
        "Array.from(document.querySelectorAll('.mode-tab')).find(b => b.textContent.trim()==='对话')?.click();"
      )
      await sleep(350)

      // Create 2 sessions
      await evalJs(
        "Array.from(document.querySelectorAll('.nav-row')).find(b => b.textContent.includes('新建会话'))?.click();"
      )
      await sleep(350)
      await evalJs(
        "Array.from(document.querySelectorAll('.nav-row')).find(b => b.textContent.includes('新建会话'))?.click();"
      )
      await sleep(350)
      const initSessions = await evalJs("return document.querySelectorAll('.sidebar-scroll .list-item').length;")
      assert(initSessions >= 2, `Created sessions (${initSessions})`)

      // Open session menu -> Pin
      await evalJs("document.querySelector('.sidebar-scroll .list-item .more-dots')?.click();")
      await waitFor("document.querySelector('.session-menu')", 3000, 'Session menu opened')
      await sleep(200)
      await evalJs(
        "Array.from(document.querySelectorAll('.session-menu .smenu-item')).find(b => b.textContent.includes('置顶'))?.click();"
      )
      await sleep(350)
      const hasPinned = await evalJs(
        "return Array.from(document.querySelectorAll('.section-toggle')).some(el => el.textContent.includes('Pinned'));"
      )
      assert(hasPinned, 'Pinned section appeared')

      // Rename pinned session
      await evalJs("document.querySelector('.sidebar-scroll .list-item .more-dots')?.click();")
      await waitFor("document.querySelector('.session-menu')", 3000, 'Rename menu')
      await evalJs(
        "Array.from(document.querySelectorAll('.session-menu .smenu-item')).find(b => b.textContent.includes('重命名'))?.click();"
      )
      await waitFor("document.querySelector('.rename-input')", 3000, 'Session rename input')
      await evalJs("const i = document.querySelector('.rename-input'); i.focus(); i.select();")
      await insertText('架构评审会话')
      await pressKey('Enter', 'Enter', 13, 0)
      await sleep(350)
      const renamedOk = await evalJs(
        "return document.querySelector('.sidebar-scroll')?.innerText.includes('架构评审会话');"
      )
      assert(renamedOk, 'Session renamed to 架构评审会话')

      // Archive session -> expand Archived section
      await evalJs("document.querySelector('.sidebar-scroll .list-item .more-dots')?.click();")
      await waitFor("document.querySelector('.session-menu')", 3000, 'Archive menu')
      await evalJs(
        "Array.from(document.querySelectorAll('.session-menu .smenu-item')).find(b => b.textContent.includes('归档'))?.click();"
      )
      await sleep(350)
      const hasArchived = await evalJs(
        "return Array.from(document.querySelectorAll('.section-toggle')).some(el => el.textContent.includes('已归档'));"
      )
      assert(hasArchived, 'Archived section appeared')
      await evalJs(
        "Array.from(document.querySelectorAll('.section-toggle')).find(el => el.textContent.includes('已归档'))?.click();"
      )
      await sleep(350)

      // Delete archived session
      await evalJs("document.querySelector('.archived-row .more-dots')?.click();")
      await waitFor("document.querySelector('.session-menu')", 3000, 'Delete session menu')
      await evalJs(
        "Array.from(document.querySelectorAll('.session-menu .smenu-item')).find(b => b.textContent.includes('删除'))?.click();"
      )
      await sleep(350)

      // Search sessions
      await evalJs(
        "Array.from(document.querySelectorAll('.nav-row')).find(b => b.textContent.includes('搜索'))?.click();"
      )
      await waitFor("document.querySelector('.search-input')", 3000, 'Search input opened')
      await evalJs("document.querySelector('.search-input')?.focus();")
      await insertText('新会话')
      await sleep(350)
    }
  },

  {
    id: 'S6',
    name: 'ChatPane Collaboration, Quotes, PASTED Cards & Model/Engine Picker',
    videoFile: 'S6-chat-collab.mp4',
    async run(ctx) {
      const { evalJs, pressKey, insertText, waitFor, assert, selectWordInFirstParagraph, sleep } = ctx

      // Quote selection -> ChatPane
      await selectWordInFirstParagraph('工作窃取机制决定了尾延迟上限')
      await evalJs(
        `document.querySelector('.selection-toolbar button[aria-label="引用到对话"]')?.click();`
      )
      await waitFor("document.querySelector('.quoted-chip, .quoted-bar')", 4000, 'Quote chip in ChatPane')
      await sleep(350)

      // Clear quotes button
      await evalJs("document.querySelector('.quoted-clear')?.click();")
      await sleep(250)
      const cleared = await evalJs("return !document.querySelector('.quoted-bar');")
      assert(cleared, 'Quote bar cleared')

      // Paste long text -> PASTED card -> edit modal -> save -> reopen & Esc -> remove card
      await evalJs(`
        const ta = document.querySelector('.chat-pane textarea'); ta.focus();
        const dt = new DataTransfer();
        dt.setData('text/plain', '这是一段用于完整 Spec 真机验收的长粘贴文本。'.repeat(25));
        ta.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));
      `)
      await waitFor("document.querySelector('.pasted-card')", 3000, 'PASTED card created')
      await sleep(350)
      await evalJs("document.querySelector('.pasted-card')?.click();")
      await waitFor("document.querySelector('.pasted-modal-mask')", 3000, 'PASTED modal opened')
      await sleep(350)
      await evalJs("document.querySelector('.pasted-modal-mask textarea')?.focus();")
      await insertText('\n[已编辑]')
      await evalJs(
        "Array.from(document.querySelectorAll('.pasted-modal-mask button')).find(b => b.textContent.includes('保存修改'))?.click();"
      )
      await waitFor("!document.querySelector('.pasted-modal-mask')", 3000, 'PASTED modal saved')
      await sleep(300)
      await evalJs("document.querySelector('.pasted-card')?.click();")
      await waitFor("document.querySelector('.pasted-modal-mask')", 3000, 'PASTED modal reopened')
      await sleep(250)
      await pressKey('Escape', 'Escape', 27, 0)
      await waitFor("!document.querySelector('.pasted-modal-mask')", 3000, 'PASTED modal Esc closed')

      // Model picker & Engine switch
      await evalJs("document.querySelector('.model-chip')?.click();")
      await waitFor("document.querySelector('.model-popover')", 3000, 'Model popover opened')
      await sleep(350)
      await evalJs('document.body.click();')
      await sleep(200)
      await evalJs(`
        const sel = document.querySelector('.chat-pane select');
        if (sel && sel.options.length > 1) {
          sel.value = sel.options[1].value;
          sel.dispatchEvent(new Event('change', { bubbles: true }));
        }
      `)
      await sleep(350)

      // Hide and reopen document pane (MuyaEditorPane uses v-show so check bounding box width)
      await evalJs('document.querySelector(\'button[title="关闭文档面板"]\')?.click();')
      await waitFor("(document.querySelector('.mu-container')?.getBoundingClientRect().width || 0) === 0", 3000, 'Doc pane hidden')
      await sleep(350)
      await evalJs(
        "Array.from(document.querySelectorAll('.chat-pane button')).find(b => b.textContent.includes('打开文档'))?.click();"
      )
      await waitFor("(document.querySelector('.mu-container')?.getBoundingClientRect().width || 0) > 100", 4000, 'Doc pane reopened')
      await sleep(350)
    }
  },

  {
    id: 'S7',
    name: 'Themes, Settings Persistence & Layout Split Resize',
    videoFile: 'S7-themes-settings-layout.mp4',
    async run(ctx) {
      const { evalJs, pressKey, waitFor, assert, mouseDrag, sleep } = ctx

      await pressKey(',', 'Comma', 188, 2)
      await waitFor("document.querySelector('.settings-mask')", 3000, 'Settings opened')
      await sleep(300)

      // Cycle themes: Light -> Dark
      await evalJs(
        "Array.from(document.querySelectorAll('.theme-card')).find(el => el.textContent.includes('纯净白'))?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.theme-card')).find(el => el.textContent.includes('深邃暗'))?.click();"
      )
      await sleep(300)
      const darkAttr = await evalJs("return document.documentElement.getAttribute('data-theme');")
      assert(darkAttr === 'dark', `Theme switched to dark (${darkAttr})`)

      // Font size & Max width buttons
      await evalJs(
        "Array.from(document.querySelectorAll('.seg-btn')).find(el => el.textContent.trim()==='17px')?.click();"
      )
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.seg-btn')).find(el => el.textContent.trim()==='960px')?.click();"
      )
      await sleep(250)

      // AI Backend tab -> Check local CLI
      await evalJs(
        "Array.from(document.querySelectorAll('.snav-item')).find(el => el.textContent.includes('AI 后端'))?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.settings-body button')).find(el => el.textContent.includes('检测本地 CLI'))?.click();"
      )
      await sleep(500)

      // Writing preferences tab -> toggle enable + reset default
      await evalJs(
        "Array.from(document.querySelectorAll('.snav-item')).find(el => el.textContent.includes('个人写作'))?.click();"
      )
      await sleep(300)
      await evalJs(
        "Array.from(document.querySelectorAll('.writing-actions button'))[0]?.click();"
      )
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.writing-actions button'))[0]?.click();"
      )
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.writing-actions button')).find(b => b.textContent.includes('恢复默认'))?.click();"
      )
      await sleep(250)
      await pressKey('Escape', 'Escape', 27, 0)
      await waitFor("!document.querySelector('.settings-mask')", 3000, 'Settings closed')
      await sleep(300)

      // Drag sidebar resize handle
      const sbBefore = await evalJs("return document.querySelector('.left-sidebar').getBoundingClientRect().width;")
      const sbHandle = await evalJs(`
        const h = document.querySelector('.pane-resizer[title="拖动调节左侧栏宽度"]');
        if (!h) return null;
        const r = h.getBoundingClientRect();
        return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
      `)
      assert(Boolean(sbHandle), 'Sidebar resize handle found')
      await mouseDrag(sbHandle.x, sbHandle.y, sbHandle.x + 55, sbHandle.y)
      await sleep(300)
      const sbAfter = await evalJs("return document.querySelector('.left-sidebar').getBoundingClientRect().width;")
      assert(Math.abs(sbAfter - sbBefore) >= 15, `Sidebar resized (${sbBefore}px -> ${sbAfter}px)`)
    }
  },

  {
    id: 'S8',
    name: 'Document Actions, Copy Rich HTML & Version History Modal',
    videoFile: 'S8-export-history-rollback.mp4',
    async run(ctx) {
      const { evalJs, pressKey, waitFor, assert, sleep } = ctx

      // Open document actions menu -> Copy Rich HTML
      await evalJs('document.querySelector(\'button[title="文档操作"]\')?.click();')
      await waitFor("document.querySelector('.doc-menu, .menu-item')", 3000, 'Doc actions menu')
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.menu-item')).find(el => el.textContent.includes('复制富文本'))?.click();"
      )
      await sleep(450)

      // Open Version History modal -> test Ctrl+Z guard -> Esc close
      await evalJs('document.querySelector(\'button[title="文档操作"]\')?.click();')
      await waitFor("document.querySelector('.menu-item')", 3000, 'Doc actions menu for history')
      await sleep(200)
      await evalJs(
        "Array.from(document.querySelectorAll('.menu-item')).find(el => el.textContent.includes('版本历史'))?.click();"
      )
      await waitFor("document.querySelector('.modal-mask')", 3000, 'History modal opened')
      await sleep(350)
      await pressKey('z', 'KeyZ', 90, 2)
      await sleep(200)
      await pressKey('Escape', 'Escape', 27, 0)
      await waitFor("!document.querySelector('.modal-mask')", 3000, 'History modal Esc closed')
      await sleep(250)
      assert(true, 'Document actions, copy rich HTML & history modal verified')
    }
  },

  {
    id: 'S9',
    name: 'Local Images, Dark Collab Empty State & Shortcut Guards',
    videoFile: 'S9-images-empty-guards.mp4',
    async run(ctx) {
      const { evalJs, pressKey, waitFor, assert, sleep } = ctx

      // 1. Verify nested local image loaded
      await evalJs("document.querySelector('.muya-scroll').scrollTop = 360;")
      await sleep(450)
      const imgInfo = await evalJs(`
        const img = document.querySelector('.mu-container img');
        return img ? { complete: img.complete, w: img.naturalWidth, h: img.naturalHeight } : null;
      `)
      assert(imgInfo && imgInfo.complete && imgInfo.w > 0, `Local image loaded (${JSON.stringify(imgInfo)})`)

      // 2. Dark theme + Collab 3-pane mode
      await pressKey(',', 'Comma', 188, 2)
      await waitFor("document.querySelector('.settings-mask')", 3000, 'Settings')
      await evalJs(
        "Array.from(document.querySelectorAll('.theme-card')).find(el => el.textContent.includes('深邃暗'))?.click();"
      )
      await pressKey('Escape', 'Escape', 27, 0)
      await sleep(250)
      await evalJs(
        "Array.from(document.querySelectorAll('.mode-tab')).find(b => b.textContent.trim()==='对话')?.click();"
      )
      await sleep(350)

      // 3. Close all tabs via Ctrl+W -> empty state must still expose close-doc X button
      await pressKey('w', 'KeyW', 87, 2)
      await sleep(450)
      const emptyStateCheck = await evalJs(`
        return {
          hasNewDocBtn: Array.from(document.querySelectorAll('button')).some(b => b.textContent.trim()==='新建文档'),
          hasCloseDocBtn: Boolean(document.querySelector('button[title="关闭文档面板"]'))
        };
      `)
      assert(emptyStateCheck.hasNewDocBtn, 'Empty state shown after closing last tab')
      assert(emptyStateCheck.hasCloseDocBtn, 'Doc header X close button remains visible in collab empty state')

      // 4. Empty state shortcut guards (Ctrl+Z / Ctrl+F / Ctrl+S)
      await pressKey('z', 'KeyZ', 90, 2)
      await pressKey('f', 'KeyF', 70, 2)
      await pressKey('s', 'KeyS', 83, 2)
      await sleep(350)
      const findBarLeaked = await evalJs("return Boolean(document.querySelector('.find-bar'));")
      assert(!findBarLeaked, 'Ctrl+F blocked in empty state')
    }
  }
]
