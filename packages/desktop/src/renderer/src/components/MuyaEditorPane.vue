<template>
  <main class="doc-pane">
    <!-- Single Unified Document Tab Bar -->
    <header class="doc-header" v-if="store.openTabs.length || (!canClose && !sidebarVisible)">
      <div class="doc-header-left">
        <button
          v-if="!canClose && !sidebarVisible"
          class="icon-btn"
          title="展开侧栏"
          @click="$emit('expand-sidebar')"
        >
          <PanelLeftOpen :size="16" :stroke-width="1.75" />
        </button>

        <div class="tab-pills" role="tablist">
          <div
            v-for="tab in store.openTabs"
            :key="tab"
            class="tab-pill"
            :class="{ active: tab === store.activeDocRelPath }"
            role="tab"
            :aria-selected="tab === store.activeDocRelPath"
            @click="store.openDocument(tab)"
          >
            <span class="tab-label">{{ tab.replace(/\.md$/i, '') }}</span>
            <button
              class="tab-close"
              title="关闭标签"
              @click.stop="store.closeTab(tab)"
            >
              ×
            </button>
          </div>
        </div>
      </div>

      <div class="doc-header-right" style="position: relative">
        <span v-if="saveHint" class="save-hint">{{ saveHint }}</span>

        <button
          v-if="store.activeDocRelPath"
          class="icon-btn"
          title="文档操作"
          @click.stop="menuOpen = !menuOpen"
        >
          <MoreHorizontal :size="16" :stroke-width="1.75" />
        </button>

        <div v-if="menuOpen" class="dropdown-menu" @click.stop>
          <button class="menu-item" @click="saveNow(true)">保存 (Ctrl+S)</button>
          <button class="menu-item" @click="doExport('md')">另存为 Markdown...</button>
          <button class="menu-item" @click="openHistory">版本历史</button>
          <div class="menu-sep"></div>
          <button class="menu-item" @click="doExport('html')">导出 HTML</button>
          <button class="menu-item" @click="doExport('pdf')">导出 PDF</button>
          <button class="menu-item" @click="copyRichHtml">复制富文本</button>
        </div>

        <button v-if="canClose" class="icon-btn" title="关闭文档面板" @click="$emit('close-doc')">
          <X :size="15" :stroke-width="1.75" />
        </button>
      </div>
    </header>

    <!-- Clean Empty State -->
    <div v-if="!store.activeDocRelPath" class="empty-doc">
      <button class="btn-outline" @click="createBlankDoc">新建文档</button>
    </div>

    <!-- Lightweight Code / Plaintext Preview for non-Markdown Workspace Files -->
    <div v-else-if="!isMarkdownFile" class="code-preview-scroll">
      <pre class="code-preview-box"><code>{{ store.markdown }}</code></pre>
    </div>

    <!-- Muya WYSIWYG Editorial Paper + Right Margin Comment Indicators -->
    <div
      v-show="store.activeDocRelPath && isMarkdownFile"
      ref="scrollEl"
      class="muya-scroll"
      @mouseup="handleSelectionMouseUp"
      @mousedown="handleEditorMouseDown"
      @wheel.passive="onUserScroll"
      @touchstart.passive="onUserScroll"
    >
      <div ref="hostWrapEl" class="muya-host">
        <!-- Agent Active Line / Block Highlight Overlay (HUD + Laser Scan + Live Typewriter) -->
        <div
          v-if="animState.visible"
          class="agent-highlight-box"
          :class="[animState.phase]"
          :style="{
            top: animState.boxTop + 'px',
            left: animState.boxLeft + 'px',
            width: animState.boxWidth + 'px',
            height: animState.boxHeight + 'px'
          }"
        >
          <span class="hud-corner tl" />
          <span class="hud-corner tr" />
          <span class="hud-corner bl" />
          <span class="hud-corner br" />
          <div v-if="animState.phase === 'writing'" class="agent-scan-beam" />
          <div v-if="animState.phase === 'settled'" class="agent-settled-sheen" />
          <div
            v-if="animState.showStream && animState.streamText"
            ref="streamPreviewEl"
            class="agent-stream-preview"
          >
            <span>{{ animState.streamText }}</span><span class="agent-caret" />
          </div>
          <div
            v-if="animState.phase === 'writing'"
            class="agent-progress-track"
          >
            <div
              class="agent-progress-fill"
              :style="{ width: animState.progress + '%' }"
            />
          </div>
        </div>

        <!-- Interactive Floating Agent Cursor -->
        <div
          v-if="animState.visible"
          class="agent-floating-cursor"
          :class="{
            pinned: animState.pinned,
            settled: animState.phase === 'settled',
            fading: animState.phase === 'fading'
          }"
          :style="{
            transform: `translate(${animState.cursorX}px, ${animState.cursorY}px)`
          }"
          @click.stop="animState.pinned = !animState.pinned"
        >
          <div class="agent-cursor-pill">
            <span class="agent-cursor-dot" />
            <span class="agent-cursor-label">{{ animState.label }}</span>
            <span v-if="animState.badge" class="agent-cursor-badge">{{ animState.badge }}</span>
          </div>
          <div class="agent-cursor-inspector" @click.stop>
            <div style="color: #d6d3cb; margin-bottom: 8px">{{ animState.detail }}</div>
            <div style="display: flex; gap: 6px">
              <button @click="togglePause">{{ animState.paused ? '继续' : '暂停' }}</button>
              <button @click="skipCurrent">跳过</button>
              <button @click="abortAnimation(true)">终止</button>
            </div>
          </div>
        </div>

        <!-- Subtle Margin Comment Pills (Claude Docs style, no permanent 4th column!) -->
        <div class="margin-gutter">
          <button
            v-for="group in marginThreadGroups"
            :key="group.thread.id"
            class="margin-pill"
            :style="{ top: group.top + 'px' }"
            @click.stop="openThreadPopover(group.thread, $event)"
          >
            <MessageSquare :size="13" :stroke-width="1.75" />
            <span>{{ group.thread.messages.length }}</span>
          </button>
        </div>

        <!-- Real @muyajs/core Mount Point -->
        <div ref="muyaContainerEl" />
      </div>
    </div>

    <!-- Claude Docs Style Floating Selection Toolbar (Above Selection + Dark Shortcut Hints) -->
    <div
      v-if="selToolbar.open"
      class="selection-toolbar"
      :style="{ top: selToolbar.top + 'px', left: selToolbar.left + 'px' }"
      @mousedown.prevent
      @click.stop
    >
      <button
        class="sel-btn"
        aria-label="批注"
        @click="openCommentFromSelection"
      >
        <MessageSquarePlus :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>批注</span><kbd>Ctrl+Alt+M</kbd></span>
      </button>

      <button
        class="sel-btn"
        aria-label="引用到对话"
        @click="quoteSelectionToChat"
      >
        <Quote :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>引用到对话</span><kbd>Ctrl+Shift+L</kbd></span>
      </button>

      <span class="sel-sep" />

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="加粗"
        @click="applyInlineFormat('strong')"
      >
        <Bold :size="15" :stroke-width="1.85" />
        <span class="sel-tip"><span>加粗</span><kbd>Ctrl+B</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="斜体"
        @click="applyInlineFormat('em')"
      >
        <Italic :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>斜体</span><kbd>Ctrl+I</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="下划线"
        @click="applyInlineFormat('u')"
      >
        <Underline :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>下划线</span><kbd>Ctrl+U</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="删除线"
        @click="applyInlineFormat('del')"
      >
        <Strikethrough :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>删除线</span><kbd>Ctrl+D</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="行内代码"
        @click="applyInlineFormat('inline_code')"
      >
        <Code :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>行内代码</span><kbd>Ctrl+E</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="高亮"
        @click="applyInlineFormat('mark')"
      >
        <Highlighter :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>高亮</span><kbd>Ctrl+Shift+H</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="链接"
        @click="applyInlineFormat('link')"
      >
        <Link :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>链接</span><kbd>Ctrl+L</kbd></span>
      </button>

      <button
        class="sel-btn"
        :disabled="!selToolbar.canFormat"
        aria-label="清除格式"
        @click="applyInlineFormat('clear')"
      >
        <RemoveFormatting :size="15" :stroke-width="1.75" />
        <span class="sel-tip"><span>清除格式</span><kbd>Ctrl+Shift+R</kbd></span>
      </button>
    </div>

    <!-- Floating New Comment Composer (Right Side, Non-modal) -->
    <div
      v-if="floatComment.open"
      class="right-float-comment"
      :style="{ top: floatComment.top + 'px', left: floatComment.left + 'px' }"
      @click.stop
    >
      <div class="float-head">
        <span>批注</span>
        <button class="close-x" @click="floatComment.open = false">✕</button>
      </div>
      <div class="float-quote">“{{ floatComment.quote }}”</div>
      <textarea
        ref="floatInputEl"
        v-model="floatComment.text"
        rows="3"
        placeholder="提问或修改指令..."
        class="float-textarea"
        @keydown="onFloatKeydown"
      />
      <div class="float-foot">
        <button class="btn-primary" @click="submitFloatComment">发送</button>
      </div>
    </div>

    <!-- Floating Existing Thread Popover (Click margin pill to view/reply/resolve) -->
    <div
      v-if="activeThread"
      class="right-float-comment"
      :style="{ top: threadPop.top + 'px', left: threadPop.left + 'px' }"
      @click.stop
    >
      <div class="float-head">
        <span>{{ activeThread.createdBy === 'agent' ? 'Agent 提问' : '批注' }}</span>
        <div style="display: flex; gap: 4px">
          <button
            v-if="!activeThread.resolved"
            class="resolve-btn"
            @click="resolveActiveThread"
          >
            解决
          </button>
          <button class="close-x" @click="threadPop.threadId = null">✕</button>
        </div>
      </div>
      <div class="float-quote">“{{ activeThread.quote }}”</div>
      <div class="thread-msgs">
        <div v-for="m in activeThread.messages" :key="m.id" class="thread-msg-row">
          <span class="msg-who">{{ m.who === 'user' ? '你' : 'AGY' }}</span>
          <div class="msg-body">{{ m.text }}</div>
        </div>
      </div>
      <div v-if="!activeThread.resolved" class="thread-reply-bar">
        <textarea
          v-model="threadPop.reply"
          rows="1"
          placeholder="回复..."
          class="reply-input"
          @keydown="onThreadReplyKeydown"
        />
        <button class="btn-primary" style="padding: 4px 10px" @click="submitThreadReply">发送</button>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import {
  Bold,
  Code,
  Highlighter,
  Italic,
  Link,
  MessageSquare,
  MessageSquarePlus,
  MoreHorizontal,
  PanelLeftOpen,
  Quote,
  RemoveFormatting,
  Strikethrough,
  Underline,
  X
} from 'lucide-vue-next'
import {
  CodeBlockLanguageSelector,
  EmojiSelector,
  LinkTools,
  MarkdownToHtml,
  Muya,
  ParagraphFrontButton,
  ParagraphFrontMenu,
  ParagraphQuickInsertMenu,
  PreviewToolBar,
  TableChessboard,
  TableColumnToolbar,
  TableRowColumMenu,
  zhCN
} from '@muyajs/core'
import type { CommentThread } from '@shared/types'
import { useAppStore } from '@/stores/appStore'

withDefaults(
  defineProps<{
    sidebarVisible?: boolean
    canClose?: boolean
  }>(),
  {
    sidebarVisible: true,
    canClose: true
  }
)

const emit = defineEmits<{
  (e: 'expand-sidebar'): void
  (e: 'close-doc'): void
  (e: 'open-history'): void
}>()

const store = useAppStore()
const scrollEl = ref<HTMLElement | null>(null)
const hostWrapEl = ref<HTMLElement | null>(null)
const muyaContainerEl = ref<HTMLElement | null>(null)
const streamPreviewEl = ref<HTMLElement | null>(null)
const floatInputEl = ref<HTMLTextAreaElement | null>(null)

const menuOpen = ref(false)
const saveHint = ref('')
let saveDebounceTimer: ReturnType<typeof setTimeout> | null = null
let saveHintTimer: ReturnType<typeof setTimeout> | null = null
let muyaInstance: Muya | null = null
let suppressMuyaChange = false
let pluginsRegistered = false

const isMarkdownFile = computed(
  () => !store.activeDocRelPath || /\.(md|markdown)$/i.test(store.activeDocRelPath)
)

const selToolbar = reactive({
  open: false,
  top: 100,
  left: 400,
  anchorY: 140,
  blockIndex: 0,
  quote: '',
  canFormat: true
})

const floatComment = reactive({
  open: false,
  top: 120,
  left: 600,
  blockIndex: 0,
  quote: '',
  text: ''
})

const threadPop = reactive({
  threadId: null as string | null,
  top: 120,
  left: 600,
  reply: ''
})

const activeThread = computed(
  () => store.threads.find((t) => t.id === threadPop.threadId) ?? null
)

const marginTick = ref(0)
const marginThreadGroups = computed(() => {
  void marginTick.value
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  const openThreads = store.threads.filter((t) => !t.resolved)
  const raw = openThreads.map((thread, idx) => {
    const el = els[Math.min(Math.max(0, thread.blockIndex), Math.max(0, els.length - 1))]
    let top = 24 + idx * 44
    if (wrap && el) {
      top = Math.max(12, el.getBoundingClientRect().top - wrap.getBoundingClientRect().top + 4)
    }
    return { thread, top }
  })
  // Prevent vertical overlap when multiple comments anchor on the same paragraph
  raw.sort((a, b) => a.top - b.top)
  let lastTop = -Infinity
  const minGap = 32
  return raw.map((item) => {
    const adjusted = item.top < lastTop + minGap ? lastTop + minGap : item.top
    lastTop = adjusted
    return { thread: item.thread, top: adjusted }
  })
})

const animState = reactive({
  runId: 0,
  running: false,
  visible: false,
  paused: false,
  skip: false,
  pinned: false,
  activeBlockIndex: -1,
  phase: 'lock' as 'lock' | 'writing' | 'settled' | 'fading',
  progress: 0,
  label: '',
  badge: '',
  detail: '',
  streamText: '',
  showStream: false,
  boxTop: 0,
  boxLeft: 0,
  boxWidth: 0,
  boxHeight: 0,
  cursorX: 0,
  cursorY: 0
})

let userScrollPausedUntil = 0
function onUserScroll() {
  userScrollPausedUntil = Date.now() + 3500
}

function ensureMuyaPlugins() {
  if (pluginsRegistered) return
  pluginsRegistered = true
  Muya.use(EmojiSelector)
  Muya.use(CodeBlockLanguageSelector)
  Muya.use(LinkTools)
  Muya.use(ParagraphFrontButton)
  Muya.use(ParagraphFrontMenu)
  Muya.use(ParagraphQuickInsertMenu)
  Muya.use(TableChessboard)
  Muya.use(TableColumnToolbar)
  Muya.use(TableRowColumMenu)
  Muya.use(PreviewToolBar)
}

function mountMuya(initialMd: string) {
  if (!muyaContainerEl.value) return
  if (muyaInstance) {
    muyaInstance.destroy()
    muyaInstance = null
  }
  muyaContainerEl.value.innerHTML = ''
  const target = document.createElement('div')
  muyaContainerEl.value.appendChild(target)

  ensureMuyaPlugins()
  muyaInstance = new Muya(target, {
    markdown: initialMd || '',
    locale: zhCN,
    frontMatter: true,
    texMathGfm: true,
    autoPairBracket: true,
    autoPairMarkdownSyntax: true,
    autoPairQuote: true
  })
  muyaInstance.init()

  muyaInstance.on('json-change', () => {
    if (suppressMuyaChange || !muyaInstance) return
    const md = muyaInstance.getMarkdown()
    // Sync in-memory live doc immediately, debounce disk write (300ms) to avoid concurrent writeFile storms
    store.onEditorMarkdownChanged(md, false)
    saveHint.value = '保存中...'
    if (saveDebounceTimer) clearTimeout(saveDebounceTimer)
    saveDebounceTimer = setTimeout(async () => {
      await store.onEditorMarkdownChanged(md, true)
      flashSavedHint()
    }, 300)
    refreshToc()
    marginTick.value += 1
  })

  refreshToc()
  nextTick(() => {
    marginTick.value += 1
  })
}

function refreshToc() {
  if (!muyaInstance) {
    store.toc = []
    return
  }
  const raw = muyaInstance.getTOC()
  store.toc = raw.map((item) => ({
    lvl: item.lvl,
    content: item.content,
    slug: item.slug
  }))
}

function setMuyaMarkdownQuietly(md: string) {
  if (!muyaInstance) {
    mountMuya(md)
    return
  }
  suppressMuyaChange = true
  try {
    muyaInstance.setContent(md)
  } finally {
    suppressMuyaChange = false
  }
  refreshToc()
  marginTick.value += 1
}

watch(
  () => [store.activeDocRelPath, store.externalDocSeq] as const,
  async ([newPath]) => {
    abortAnimation()
    selToolbar.open = false
    floatComment.open = false
    threadPop.threadId = null
    if (!newPath) return
    if (!/\.(md|markdown)$/i.test(newPath)) {
      store.toc = []
      return
    }
    await nextTick()
    if (!muyaInstance) {
      mountMuya(store.markdown)
    } else {
      setMuyaMarkdownQuietly(store.markdown)
    }
  }
)

watch(
  () => store.threads.length,
  () => {
    nextTick(() => {
      marginTick.value += 1
    })
  }
)

watch(
  () => store.scrollHeadingRequest.seq,
  () => {
    const req = store.scrollHeadingRequest
    if (!muyaContainerEl.value || !scrollEl.value) return
    const headings = Array.from(
      muyaContainerEl.value.querySelectorAll('h1, h2, h3, h4, h5, h6')
    ) as HTMLElement[]
    if (!headings.length) return

    const stripMdPrefix = (s: string) => s.replace(/^#+\s*/, '').trim()
    let match: HTMLElement | undefined
    if (req.index >= 0 && req.index < headings.length) {
      const candidate = headings[req.index]
      if (!req.text || stripMdPrefix(candidate.textContent || '') === req.text.trim()) {
        match = candidate
      }
    }
    if (!match && req.text) {
      match = headings.find((h) => stripMdPrefix(h.textContent || '') === req.text.trim())
    }
    if (!match) return

    const containerRect = scrollEl.value.getBoundingClientRect()
    const targetRect = match.getBoundingClientRect()
    const nextScrollTop = Math.max(0, scrollEl.value.scrollTop + (targetRect.top - containerRect.top) - 24)
    scrollEl.value.scrollTo({ top: nextScrollTop, behavior: 'smooth' })
  }
)

watch(
  () => store.pendingDocOps.length,
  () => {
    if (!animState.running && store.pendingDocOps.length > 0) {
      drainOpQueue()
    }
  }
)

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

function splitTopBlocks(md: string): string[] {
  const lines = md.replace(/\r\n/g, '\n').split('\n')
  const blocks: string[] = []
  let buf: string[] = []
  let inFence = false
  const flush = () => {
    const t = buf.join('\n').trim()
    if (t) blocks.push(t)
    buf = []
  }
  for (const line of lines) {
    if (/^```/.test(line.trim())) {
      inFence = !inFence
      buf.push(line)
      continue
    }
    if (!inFence && line.trim() === '') {
      flush()
      continue
    }
    buf.push(line)
  }
  flush()
  return blocks
}

function getTopBlockElements(): HTMLElement[] {
  const editorRoot = muyaContainerEl.value?.querySelector('.mu-container')
  if (!editorRoot) return []
  return Array.from(editorRoot.children) as HTMLElement[]
}

function scrollBlockToGoldenZone(target: HTMLElement) {
  const scroller = scrollEl.value
  if (!scroller || Date.now() < userScrollPausedUntil) return
  const sRect = scroller.getBoundingClientRect()
  const tRect = target.getBoundingClientRect()
  const relTop = tRect.top - sRect.top
  const viewH = scroller.clientHeight
  // Keep calm if already inside the comfortable 18%..60% reading band
  if (relTop >= viewH * 0.18 && relTop <= viewH * 0.6) return
  const desiredTop = scroller.scrollTop + relTop - viewH * 0.38
  scroller.scrollTo({
    top: Math.max(0, desiredTop),
    behavior: 'smooth'
  })
}

function positionOverlayOnBlock(
  blockIndex: number,
  label: string,
  detail: string,
  shouldScroll = true
) {
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  const target = els[Math.min(Math.max(0, blockIndex), Math.max(0, els.length - 1))]
  if (!wrap || !target) return

  if (shouldScroll) {
    scrollBlockToGoldenZone(target)
  }
  const wRect = wrap.getBoundingClientRect()
  const tRect = target.getBoundingClientRect()

  animState.boxTop = Math.max(0, tRect.top - wRect.top - 6)
  animState.boxLeft = Math.max(0, tRect.left - wRect.left - 10)
  animState.boxWidth = tRect.width + 20
  animState.boxHeight = Math.max(36, tRect.height + 12)
  // Dock cursor pill neatly on the top-right edge of the highlighted block so it never overlaps headings above
  animState.cursorX = Math.max(animState.boxLeft + 8, animState.boxLeft + animState.boxWidth - 186)
  animState.cursorY = Math.max(0, animState.boxTop - 15)
  animState.label = label
  animState.detail = detail
  animState.visible = true
}

function formatReadableStreamText(rawMd: string): string {
  return rawMd
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .trim()
}

function togglePause() {
  animState.paused = !animState.paused
}

function skipCurrent() {
  animState.skip = true
}

function abortAnimation(persistHumanContent = false) {
  const wasRunning = animState.running
  animState.runId += 1
  animState.running = false
  animState.visible = false
  animState.pinned = false
  animState.activeBlockIndex = -1
  // Drop remaining queued ops for the active doc so they don't overwrite the human's edit
  if (store.activeDocRelPath) {
    store.pendingDocOps = store.pendingDocOps.filter(
      (item) => item.op.docRelPath !== store.activeDocRelPath
    )
  }
  if (persistHumanContent && wasRunning && muyaInstance && store.activeDocRelPath) {
    void store.onEditorMarkdownChanged(muyaInstance.getMarkdown(), true)
  }
}

function handleEditorMouseDown(e: MouseEvent) {
  if (!animState.running || animState.activeBlockIndex < 0) return
  const els = getTopBlockElements()
  const activeEl = els[animState.activeBlockIndex]
  if (activeEl && activeEl.contains(e.target as Node)) {
    abortAnimation(true)
  }
}

async function drainOpQueue() {
  const myRunId = ++animState.runId
  animState.running = true
  animState.paused = false

  while (store.pendingDocOps.length > 0) {
    if (animState.runId !== myRunId) return
    const item = store.shiftPendingOp()
    if (!item) break
    const op = item.op
    if (
      op.type !== 'replace_block' &&
      op.type !== 'insert_block' &&
      op.type !== 'delete_block'
    ) {
      continue
    }
    // Critical guard: never apply another document's block op onto the currently open tab
    if (op.docRelPath !== store.activeDocRelPath) {
      continue
    }

    animState.skip = false
    const currentMd = muyaInstance ? muyaInstance.getMarkdown() : store.markdown
    const blocks = splitTopBlocks(currentMd)

    const targetIdx =
      op.type === 'insert_block'
        ? Math.min(blocks.length, Math.max(0, op.afterBlockIndex + 1))
        : Math.min(Math.max(0, op.blockIndex), Math.max(0, blocks.length - 1))

    animState.activeBlockIndex = targetIdx
    animState.phase = 'lock'
    animState.progress = 0
    animState.streamText = ''
    animState.showStream = false
    animState.badge = `§${targetIdx + 1}`
    positionOverlayOnBlock(targetIdx, op.label, `锁定第 ${targetIdx + 1} 段`, true)
    await sleep(420)
    while (animState.paused && animState.runId === myRunId) await sleep(80)
    if (animState.runId !== myRunId) return

    if ((op.type === 'replace_block' || op.type === 'insert_block') && !animState.skip) {
      const readable = formatReadableStreamText(op.newMarkdown)
      const chars = Array.from(readable)
      const totalChars = chars.length
      // Adaptive cinematic pacing: 1.0s ~ 2.0s depending on length (slightly faster when queue has backlog)
      const backlogFactor = store.pendingDocOps.length > 1 ? 0.72 : 1
      const totalSteps = Math.min(44, Math.max(22, Math.ceil(totalChars / 4)))
      const stepDelay = Math.round(36 * backlogFactor)

      animState.phase = 'writing'
      animState.showStream = totalChars > 0
      for (let step = 1; step <= totalSteps; step++) {
        if (animState.runId !== myRunId || animState.skip) break
        while (animState.paused && animState.runId === myRunId) await sleep(80)
        const ratio = step / totalSteps
        // Ease-out curve for crisp, responsive feel
        const eased = 1 - Math.pow(1 - ratio, 1.35)
        const count = Math.min(totalChars, Math.max(1, Math.round(totalChars * eased)))
        const pct = Math.round(ratio * 100)
        animState.streamText = chars.slice(0, count).join('')
        animState.progress = pct
        animState.badge = `${pct}%`
        animState.detail = `正在改写第 ${targetIdx + 1} 段 (${pct}%)`
        await nextTick()
        if (streamPreviewEl.value) {
          const neededH = streamPreviewEl.value.scrollHeight + 14
          if (neededH > animState.boxHeight) {
            animState.boxHeight = neededH
          }
        }
        await sleep(stepDelay)
      }
    } else if (op.type === 'delete_block' && !animState.skip) {
      animState.phase = 'writing'
      animState.badge = 'DEL'
      animState.progress = 100
      await sleep(520)
    }

    if (animState.runId !== myRunId) return

    animState.showStream = false
    animState.streamText = ''

    if (op.resultingMarkdown !== undefined) {
      // Authoritative snapshot already persisted by Main Process DocMcpServer;
      // NEVER write it back to main/disk here to avoid racing with subsequent MCP calls.
      store.markdown = op.resultingMarkdown
      setMuyaMarkdownQuietly(op.resultingMarkdown)
    } else {
      if (op.type === 'delete_block') {
        blocks.splice(targetIdx, 1)
      } else if (op.type === 'insert_block') {
        blocks.splice(targetIdx, 0, op.newMarkdown)
      } else {
        blocks[targetIdx] = op.newMarkdown
      }
      const nextMd = blocks.join('\n\n') + '\n'
      setMuyaMarkdownQuietly(nextMd)
      await store.onEditorMarkdownChanged(nextMd, true)
    }

    await nextTick()
    animState.phase = 'settled'
    animState.progress = 100
    animState.badge = '✓'
    positionOverlayOnBlock(targetIdx, op.label, '已完成更新', false)
    await sleep(store.pendingDocOps.length > 0 ? 520 : 880)
  }

  if (animState.runId === myRunId) {
    animState.phase = 'fading'
    await sleep(520)
    if (animState.runId === myRunId) {
      animState.running = false
      animState.visible = false
      animState.activeBlockIndex = -1
      animState.showStream = false
      animState.streamText = ''
    }
  }
}

async function createBlankDoc() {
  if (!store.activeWorkspace) return
  const relPath = `未命名-${Date.now().toString().slice(-4)}.md`
  const today = new Date().toISOString().slice(0, 10)
  const initial = `# 未命名文档\n\n> ${today} · 你\n\n`
  await window.mark5.writeMarkdownFile(store.activeWorkspace.rootPath, relPath, initial)
  await store.refreshFiles()
  await store.openDocument(relPath)
}

function computePopoverPos(anchorY: number, cardHeight: number) {
  const cardWidth = 264
  const hostRight = hostWrapEl.value?.getBoundingClientRect().right ?? window.innerWidth - cardWidth - 28
  const rightSpace = window.innerWidth - hostRight
  // Prefer docking right next to the document paper edge (never covering text) when margin space is sufficient
  const left =
    rightSpace >= cardWidth + 24
      ? Math.round(hostRight + 12)
      : Math.max(24, window.innerWidth - cardWidth - 16)
  const top = Math.min(
    Math.max(64, window.innerHeight - cardHeight - 16),
    Math.max(64, Math.round(anchorY - 12))
  )
  return { top, left }
}

function findBlockIndexForNode(node: Node | null): number {
  const els = getTopBlockElements()
  let cur: Node | null = node
  while (cur && cur !== muyaContainerEl.value) {
    const idx = els.indexOf(cur as HTMLElement)
    if (idx >= 0) return idx
    cur = cur.parentNode
  }
  return 0
}

function captureCurrentSelection(): boolean {
  const sel = window.getSelection()
  if (!sel || sel.isCollapsed || !sel.toString().trim() || sel.rangeCount === 0) {
    selToolbar.open = false
    return false
  }
  if (!muyaContainerEl.value || !muyaContainerEl.value.contains(sel.anchorNode)) {
    selToolbar.open = false
    return false
  }
  const range = sel.getRangeAt(0)
  const rect = range.getBoundingClientRect()
  if (!rect || (rect.width === 0 && rect.height === 0)) {
    selToolbar.open = false
    return false
  }

  const quote = sel.toString().trim().slice(0, 120)
  const startBlock = findBlockIndexForNode(sel.anchorNode)
  const endBlock = findBlockIndexForNode(sel.focusNode)
  const toolbarWidth = 344
  const idealLeft = rect.left + rect.width / 2 - toolbarWidth / 2
  const left = Math.min(
    Math.max(16, window.innerWidth - toolbarWidth - 16),
    Math.max(16, Math.round(idealLeft))
  )
  const top = rect.top > 96 ? Math.round(rect.top - 48) : Math.round(rect.bottom + 10)

  selToolbar.quote = quote
  selToolbar.blockIndex = Math.min(startBlock, endBlock)
  selToolbar.anchorY = rect.top
  selToolbar.canFormat = startBlock === endBlock
  selToolbar.left = left
  selToolbar.top = top
  selToolbar.open = true
  return true
}

function handleSelectionMouseUp() {
  captureCurrentSelection()
}

function openCommentFromSelection() {
  if (!selToolbar.open && !captureCurrentSelection()) return
  const { blockIndex, quote, anchorY } = selToolbar
  selToolbar.open = false
  threadPop.threadId = null
  floatComment.quote = quote
  floatComment.blockIndex = blockIndex
  floatComment.text = ''
  const pos = computePopoverPos(anchorY, 210)
  floatComment.top = pos.top
  floatComment.left = pos.left
  floatComment.open = true
  nextTick(() => {
    floatInputEl.value?.focus()
  })
}

function quoteSelectionToChat() {
  if (!selToolbar.open && !captureCurrentSelection()) return
  const quote = selToolbar.quote
  selToolbar.open = false
  store.quoteToChat(quote)
}

function applyInlineFormat(
  type: 'strong' | 'em' | 'u' | 'del' | 'inline_code' | 'mark' | 'link' | 'clear'
) {
  if (!muyaInstance) return
  try {
    const editor = muyaInstance.editor as unknown as {
      selection?: { getSelection?: () => { anchorBlock?: { format?: (t: string) => void } } | null }
      activeContentBlock?: { format?: (t: string) => void }
    }
    const sel = editor.selection?.getSelection?.()
    const block = sel?.anchorBlock || editor.activeContentBlock
    if (block && typeof block.format === 'function') {
      block.format(type)
    }
  } finally {
    selToolbar.open = false
  }
}

function openThreadPopover(thread: CommentThread, e: MouseEvent) {
  selToolbar.open = false
  floatComment.open = false
  threadPop.threadId = thread.id
  threadPop.reply = ''
  const anchorRect = (e.currentTarget as HTMLElement | null)?.getBoundingClientRect()
  const pos = computePopoverPos(anchorRect ? anchorRect.top : e.clientY, 290)
  threadPop.top = pos.top
  threadPop.left = pos.left
}

function onThreadReplyKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
    e.preventDefault()
    submitThreadReply()
  } else if (e.key === 'Escape') {
    threadPop.threadId = null
  }
}

async function submitThreadReply() {
  if (!threadPop.threadId || !threadPop.reply.trim()) return
  const txt = threadPop.reply.trim()
  threadPop.reply = ''
  await store.replyInThread(threadPop.threadId, txt)
}

async function resolveActiveThread() {
  if (!threadPop.threadId) return
  const id = threadPop.threadId
  threadPop.threadId = null
  await store.resolveThread(id)
}

function onFloatKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
    e.preventDefault()
    submitFloatComment()
  } else if (e.key === 'Escape') {
    floatComment.open = false
  }
}

async function submitFloatComment() {
  const text = floatComment.text.trim()
  if (!text) return
  const { blockIndex, quote } = floatComment
  floatComment.open = false
  await store.sendSelectionComment(blockIndex, quote, text)
}

function openHistory() {
  menuOpen.value = false
  emit('open-history')
}

async function doExport(format: 'md' | 'html' | 'pdf') {
  menuOpen.value = false
  if (!store.activeWorkspace || !store.activeDocRelPath) return
  const md = muyaInstance ? muyaInstance.getMarkdown() : store.markdown
  let html = ''
  if (format === 'html' || format === 'pdf') {
    const converter = new MarkdownToHtml(md)
    html = await converter.generate()
  }
  const res = await window.mark5.exportDocument({
    rootPath: store.activeWorkspace.rootPath,
    docRelPath: store.activeDocRelPath,
    format,
    markdown: md,
    html
  })
  if (res.ok) store.showToast(`已导出：${res.outPath}`)
}

async function copyRichHtml() {
  menuOpen.value = false
  const md = muyaInstance ? muyaInstance.getMarkdown() : store.markdown
  const converter = new MarkdownToHtml(md)
  const html = await converter.generate()
  await navigator.clipboard.write([
    new ClipboardItem({
      'text/html': new Blob([html], { type: 'text/html' }),
      'text/plain': new Blob([md], { type: 'text/plain' })
    })
  ])
  store.showToast('已复制富文本')
}

function flashSavedHint() {
  saveHint.value = '已保存'
  if (saveHintTimer) clearTimeout(saveHintTimer)
  saveHintTimer = setTimeout(() => {
    saveHint.value = ''
  }, 1800)
}

async function saveNow(showFeedback = false) {
  menuOpen.value = false
  if (!store.activeWorkspace || !store.activeDocRelPath) return
  if (saveDebounceTimer) {
    clearTimeout(saveDebounceTimer)
    saveDebounceTimer = null
  }
  const md = muyaInstance ? muyaInstance.getMarkdown() : store.markdown
  await store.onEditorMarkdownChanged(md, true)
  flashSavedHint()
  if (showFeedback) {
    store.showToast(`已保存：${store.activeDocRelPath}`)
  }
}

const onGlobalKeydown = (e: KeyboardEvent) => {
  const mod = e.ctrlKey || e.metaKey
  const key = e.key.toLowerCase()
  if (mod && !e.altKey && !e.shiftKey && key === 's') {
    e.preventDefault()
    void saveNow(true)
    return
  }

  // Editor-scoped shortcuts (comment / quote / inline Markdown formats)
  const sel = window.getSelection()
  const inEditor = Boolean(
    muyaContainerEl.value &&
      ((sel?.anchorNode && muyaContainerEl.value.contains(sel.anchorNode)) ||
        muyaContainerEl.value.contains(document.activeElement))
  )
  if (!inEditor) return

  if (mod && e.altKey && key === 'm') {
    e.preventDefault()
    openCommentFromSelection()
    return
  }
  if (mod && e.shiftKey && key === 'l') {
    e.preventDefault()
    quoteSelectionToChat()
    return
  }
  if (mod && !e.altKey && !e.shiftKey) {
    const map: Record<string, 'strong' | 'em' | 'u' | 'del' | 'inline_code' | 'link'> = {
      b: 'strong',
      i: 'em',
      u: 'u',
      d: 'del',
      e: 'inline_code',
      l: 'link'
    }
    if (map[key]) {
      e.preventDefault()
      applyInlineFormat(map[key])
    }
  } else if (mod && e.shiftKey && !e.altKey) {
    if (key === 'h') {
      e.preventDefault()
      applyInlineFormat('mark')
    } else if (key === 'r') {
      e.preventDefault()
      applyInlineFormat('clear')
    }
  } else if (key === 'escape') {
    selToolbar.open = false
  }
}

const onDocClick = (e: MouseEvent) => {
  menuOpen.value = false
  animState.pinned = false
  const target = e.target as HTMLElement
  if (!target.closest('.selection-toolbar')) {
    const sel = window.getSelection()
    if (!sel || sel.isCollapsed || !sel.toString().trim()) {
      selToolbar.open = false
    }
  }
  if (!target.closest('.right-float-comment')) {
    const sel = window.getSelection()
    if (floatComment.open && (!sel || sel.isCollapsed)) {
      floatComment.open = false
    }
    if (threadPop.threadId) {
      threadPop.threadId = null
    }
  }
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  window.addEventListener('keydown', onGlobalKeydown)
  if (store.activeDocRelPath) {
    mountMuya(store.markdown)
  }
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  window.removeEventListener('keydown', onGlobalKeydown)
  if (saveDebounceTimer) clearTimeout(saveDebounceTimer)
  if (saveHintTimer) clearTimeout(saveHintTimer)
  if (muyaInstance) {
    muyaInstance.destroy()
    muyaInstance = null
  }
})
</script>

<style scoped>
.doc-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg-canvas);
  position: relative;
}
.doc-header {
  height: 46px;
  padding: 0 14px;
  border-bottom: 1px solid var(--border-hairline);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-shrink: 0;
}
.doc-header-left {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex: 1;
}
.save-hint {
  font-size: 11.5px;
  color: var(--ink-faint);
  white-space: nowrap;
}
.doc-header-right {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.icon-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
}
.tab-pills {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  min-width: 0;
}
.tab-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12.5px;
  padding: 4px 8px 4px 10px;
  border-radius: 7px;
  color: var(--ink-muted);
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
}
.tab-pill:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.tab-pill.active {
  background: var(--bg-subtle);
  color: var(--ink-primary);
  font-weight: 500;
}
.tab-close {
  width: 16px;
  height: 16px;
  padding: 0;
  border-radius: 4px;
  font-size: 13px;
  line-height: 1;
  color: var(--ink-faint);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
}
.tab-pill:hover .tab-close,
.tab-pill.active .tab-close {
  opacity: 1;
}
.tab-close:hover {
  background: var(--bg-active);
  color: var(--ink-primary);
}
.dropdown-menu {
  position: absolute;
  top: 34px;
  right: 0;
  z-index: 40;
  width: 176px;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  box-shadow: var(--shadow-pop);
  padding: 4px;
}
.menu-item {
  display: flex;
  width: 100%;
  padding: 7px 10px;
  text-align: left;
  border-radius: 6px;
  font-size: 13px;
}
.menu-sep {
  height: 1px;
  background: var(--border-hairline);
  margin: 4px 0;
}
.empty-doc {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.code-preview-scroll {
  flex: 1;
  overflow: auto;
  padding: 24px 36px 64px;
}
.code-preview-box {
  max-width: 960px;
  margin: 0 auto;
  padding: 18px 22px;
  background: var(--bg-sidebar);
  border: 1px solid var(--border-hairline);
  border-radius: 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 13px;
  line-height: 1.65;
  color: var(--ink-body);
  white-space: pre-wrap;
  word-break: break-word;
}
.muya-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 36px 76px 100px 48px;
  position: relative;
}
.margin-gutter {
  position: absolute;
  top: 0;
  right: -56px;
  width: 44px;
  pointer-events: none;
}
.margin-pill {
  position: absolute;
  right: 0;
  pointer-events: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 7px;
  border-radius: 8px;
  font-size: 12px;
  color: var(--ink-muted);
  background: transparent;
}
.margin-pill:hover {
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.float-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12.5px;
  font-weight: 600;
}
.close-x {
  padding: 2px 6px;
  color: var(--ink-muted);
}
.resolve-btn {
  font-size: 11.5px;
  padding: 2px 8px;
  border: 1px solid var(--border-soft);
  border-radius: 6px;
}
.float-quote {
  font-size: 12px;
  color: var(--ink-muted);
  background: var(--highlight-comment);
  border-radius: 6px;
  padding: 4px 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.float-textarea {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  resize: none;
  outline: none;
  background: var(--bg-canvas);
}
.float-textarea:focus {
  border-color: var(--ink-muted);
}
.float-foot {
  display: flex;
  justify-content: flex-end;
}
.thread-msgs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 220px;
  overflow-y: auto;
}
.thread-msg-row {
  font-size: 12.5px;
  line-height: 1.55;
  padding-top: 6px;
  border-top: 1px solid var(--border-hairline);
}
.thread-msg-row:first-child {
  border-top: none;
  padding-top: 0;
}
.msg-who {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--ink-muted);
}
.msg-body {
  color: var(--ink-body);
  margin-top: 2px;
}
.thread-reply-bar {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--border-hairline);
}
.reply-input {
  flex: 1;
  padding: 5px 8px;
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  outline: none;
  resize: none;
  line-height: 1.45;
  max-height: 96px;
}

/* Claude Docs Style Floating Selection Toolbar + Dark Shortcut Tooltip Hints */
.selection-toolbar {
  position: fixed;
  z-index: 55;
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 4px 6px;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  box-shadow: 0 8px 24px rgba(26, 25, 24, 0.12), 0 2px 6px rgba(26, 25, 24, 0.06);
}
.sel-sep {
  width: 1px;
  height: 18px;
  background: var(--border-hairline);
  margin: 0 4px;
}
.sel-btn {
  position: relative;
  width: 30px;
  height: 30px;
  padding: 0;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-primary);
  background: transparent;
}
.sel-btn:hover:not(:disabled) {
  background: var(--bg-subtle);
}
.sel-btn:disabled {
  opacity: 0.32;
  cursor: not-allowed;
}
.sel-tip {
  pointer-events: none;
  opacity: 0;
  transform: translate(-50%, 3px);
  transition: opacity 0.12s ease, transform 0.12s ease;
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 9px;
  border-radius: 7px;
  background: #1a1918;
  color: #ffffff;
  font-size: 11.5px;
  font-weight: 500;
  white-space: nowrap;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.22);
}
.sel-tip kbd {
  font-family: inherit;
  font-size: 11px;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.68);
}
.sel-btn:hover:not(:disabled) .sel-tip {
  opacity: 1;
  transform: translate(-50%, 0);
}
</style>
