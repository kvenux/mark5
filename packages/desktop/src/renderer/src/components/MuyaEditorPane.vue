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
            @mousedown.middle.prevent
            @auxclick.middle.prevent.stop="store.closeTab(tab)"
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

    <!-- Floating In-Document Find Bar (Ctrl+F) -->
    <div v-if="findBar.open" class="find-bar" @click.stop>
      <Search :size="14" :stroke-width="1.8" class="find-icon" />
      <input
        ref="findInputEl"
        v-model="findBar.query"
        class="find-input"
        type="text"
        placeholder="在文档中查找…"
        @input="runDocumentFind"
        @keydown.enter.exact.prevent="stepDocumentFind('next')"
        @keydown.enter.shift.prevent="stepDocumentFind('previous')"
        @keydown.escape.prevent="closeFindBar"
      />
      <span v-if="findBar.query" class="find-count">
        {{ findBar.total > 0 ? `${findBar.index + 1} / ${findBar.total}` : '无结果' }}
      </span>
      <button
        class="find-nav-btn"
        title="上一个匹配 (Shift+Enter)"
        :disabled="findBar.total === 0"
        @click="stepDocumentFind('previous')"
      >
        <ChevronUp :size="14" :stroke-width="1.9" />
      </button>
      <button
        class="find-nav-btn"
        title="下一个匹配 (Enter)"
        :disabled="findBar.total === 0"
        @click="stepDocumentFind('next')"
      >
        <ChevronDown :size="14" :stroke-width="1.9" />
      </button>
      <button class="find-nav-btn" title="关闭查找 (Esc)" @click="closeFindBar">
        <X :size="14" :stroke-width="1.9" />
      </button>
    </div>

    <!-- Clean Empty State -->
    <div v-if="!store.activeDocRelPath" class="empty-doc">
      <button class="btn-outline" @click="createBlankDoc">新建文档</button>
    </div>

    <!-- Lightweight Code / Plaintext Preview for non-Markdown Workspace Files -->
    <div v-else-if="!isMarkdownFile" class="code-preview-scroll">
      <pre class="code-preview-box"><code>{{ store.markdown }}</code></pre>
    </div>

    <!-- Muya WYSIWYG Editorial Paper + Right Margin Comment Cards -->
    <div
      v-show="store.activeDocRelPath && isMarkdownFile"
      ref="scrollEl"
      class="muya-scroll"
      :class="{
        'with-comment-rail': showCommentRail,
        'compact-comment-rail': showCommentRail && isCompactCommentRail
      }"
      @mouseup="handleSelectionMouseUp"
      @mousedown="handleEditorMouseDown"
      @click="activeMarginThreadId = null"
      @wheel.passive="onUserScroll"
      @touchstart.passive="onUserScroll"
    >
      <div ref="hostWrapEl" class="muya-host">
        <!-- Persistent Left-Gutter Change Bars (VS Code Git-diff style, click to review diff) -->
        <div
          v-for="bar in changedBarRects"
          :key="'chg-' + bar.changeId"
          class="changed-gutter-bar"
          :class="{ focused: activeDiffChange?.id === bar.changeId || reviewFocusIndex === bar.blockIndex }"
          :style="{ top: bar.top + 'px', left: bar.left + 'px', height: bar.height + 'px' }"
          :title="`点击审阅第 ${bar.blockIndex + 1} 段修改 Diff`"
          @click.stop="openDiffForChange(bar.changeId)"
        />

        <!-- Brief Focus Highlight when stepping through changes via Review Bar -->
        <div
          v-if="reviewFocusBox.visible"
          class="review-focus-box"
          :style="{
            top: reviewFocusBox.top + 'px',
            left: reviewFocusBox.left + 'px',
            width: reviewFocusBox.width + 'px',
            height: reviewFocusBox.height + 'px'
          }"
        />

        <!-- Form A: Floating Per-Paragraph Diff Review Card (Word-level Diff + Accept / Reject) -->
        <div
          v-if="activeDiffChange && !animState.running"
          class="diff-review-card"
          :style="{
            top: diffCardPos.top + 'px',
            left: diffCardPos.left + 'px',
            width: diffCardPos.width + 'px'
          }"
          @click.stop
        >
          <div class="drc-head">
            <div class="drc-title-wrap">
              <span class="drc-kind" :class="activeDiffChange.type">
                {{ diffKindLabel(activeDiffChange.type) }}
              </span>
              <span class="drc-title">第 {{ activeDiffChange.blockIndex + 1 }} 段改动对比</span>
            </div>
            <div class="drc-actions">
              <button
                class="drc-btn reject"
                title="拒绝并退回此段原文"
                @click="rejectChange(activeDiffChange.id)"
              >
                <RotateCcw :size="12" :stroke-width="2" />
                <span>拒绝退回</span>
              </button>
              <button
                class="drc-btn accept"
                title="接受此段修改"
                @click="acceptChange(activeDiffChange.id)"
              >
                <Check :size="12" :stroke-width="2.2" />
                <span>接受</span>
              </button>
              <button class="drc-close" title="收起 Diff 卡片" @click="activeDiffId = null">
                <X :size="13" :stroke-width="2" />
              </button>
            </div>
          </div>
          <div class="drc-body">
            <template v-for="(seg, si) in activeDiffSegments" :key="si">
              <del v-if="seg.kind === 'del'" class="diff-del">{{ seg.text }}</del>
              <ins v-else-if="seg.kind === 'add'" class="diff-add">{{ seg.text }}</ins>
              <span v-else class="diff-same">{{ seg.text }}</span>
            </template>
          </div>
        </div>

        <!-- Agent Active Edit Overlay (Restrained Typora/Zed style + Real Typing Feel) -->
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
          <div
            v-if="animState.showStream && (animState.streamHtml || animState.streamText)"
            ref="streamPreviewEl"
            class="agent-stream-preview stream-md"
            v-html="animState.streamHtml || renderStreamMd(animState.streamText)"
          />
        </div>

        <!-- Quiet Floating Agent Status Pill -->
        <div
          v-if="animState.visible"
          class="agent-floating-cursor"
          :class="{
            settled: animState.phase === 'settled',
            fading: animState.phase === 'fading'
          }"
          :style="{
            transform: `translate(${animState.cursorX}px, ${animState.cursorY}px)`
          }"
        >
          <div class="agent-cursor-pill">
            <span class="agent-cursor-dot" />
            <span class="agent-cursor-label">{{ animState.label }}</span>
            <span v-if="animState.badge" class="agent-cursor-badge">{{ animState.badge }}</span>
            <template v-if="animState.phase === 'writing' || animState.phase === 'lock'">
              <button class="pill-mini-btn" title="跳过本段动画" @click.stop="skipCurrent">
                跳过
              </button>
              <button
                v-if="animState.batchTotal > 1"
                class="pill-mini-btn"
                title="立即完成全部修改"
                @click.stop="skipAllOps"
              >
                全部完成
              </button>
            </template>
          </div>
        </div>

        <!-- Right-Margin Comment Rail (Claude Docs / Google Docs inline dialog cards when wide, compact pills when narrow) -->
        <div class="margin-gutter" :class="{ 'rail-mode': showCommentRail }">
          <template v-if="showCommentRail">
            <div
              v-for="group in marginThreadGroups"
              :key="group.thread.id"
              class="margin-thread-card"
              :class="{ active: activeMarginThreadId === group.thread.id }"
              :style="{ top: group.top + 'px' }"
              @click.stop="selectMarginThread(group.thread)"
              @mouseenter="previewCommentAnchor(group.thread)"
              @mouseleave="clearCommentAnchorPreview"
            >
              <div class="mtc-messages">
                <div
                  v-for="m in group.thread.messages"
                  :key="m.id"
                  class="mtc-msg"
                >
                  <div class="mtc-msg-head">
                    <span class="mtc-avatar" :class="m.who">
                      {{ m.who === 'user' ? 'U' : 'AI' }}
                    </span>
                    <span class="mtc-author">{{ m.who === 'user' ? '你' : activeEngineDisplayName }}</span>
                    <span class="mtc-time">{{ formatCommentTime(m.timestamp) }}</span>
                  </div>
                  <div
                    class="mtc-msg-body comment-md"
                    :class="{ collapsed: isLongComment(m.text) && !expandedCommentMsgs[m.id] }"
                    v-html="renderCommentMd(m.text)"
                  />
                  <button
                    v-if="isLongComment(m.text)"
                    class="mtc-expand-btn"
                    @click.stop="toggleCommentExpand(m.id)"
                  >
                    {{ expandedCommentMsgs[m.id] ? '收起' : '展开全文' }}
                  </button>
                </div>
              </div>

              <div v-if="activeMarginThreadId === group.thread.id" class="mtc-reply-box" @click.stop>
                <input
                  v-model="marginReplyDraft"
                  class="mtc-reply-input"
                  placeholder="回复或 @AI 继续修改…"
                  @keydown.enter.prevent="submitMarginReply(group.thread.id)"
                  @keydown.escape.prevent="activeMarginThreadId = null"
                />
                <div class="mtc-reply-actions">
                  <button
                    class="mtc-resolve-btn"
                    title="标记该批注已解决"
                    @click.stop="resolveMarginThread(group.thread.id)"
                  >
                    <Check :size="12" :stroke-width="2.1" />
                    <span>解决</span>
                  </button>
                  <button
                    class="mtc-send-btn"
                    :disabled="!marginReplyDraft.trim()"
                    @click.stop="submitMarginReply(group.thread.id)"
                  >
                    发送
                  </button>
                </div>
              </div>
            </div>
          </template>
          <template v-else>
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
          </template>
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
          <span class="msg-who">{{ m.who === 'user' ? '你' : activeEngineDisplayName }}</span>
          <div class="msg-body comment-md" v-html="renderCommentMd(m.text)" />
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

    <!-- Human-Centered Per-Turn Change Review Navigator (Step-through + Diff + Accept/Reject All) -->
    <div
      v-if="pendingChanges.length > 0 && !animState.running && isMarkdownFile"
      class="change-review-bar"
      @click.stop
    >
      <span class="crb-dot" />
      <span class="crb-label">本轮修改 {{ pendingChanges.length }} 处</span>
      <div class="crb-nav">
        <button class="crb-icon-btn" title="上一处修改" @click="stepChangedBlock(-1)">
          <ChevronLeft :size="14" :stroke-width="2" />
        </button>
        <span class="crb-counter">{{ reviewPos + 1 }}/{{ pendingChanges.length }}</span>
        <button class="crb-icon-btn" title="下一处修改" @click="stepChangedBlock(1)">
          <ChevronRight :size="14" :stroke-width="2" />
        </button>
      </div>
      <button
        class="crb-diff-btn"
        :class="{ active: Boolean(activeDiffId) }"
        title="展开或收起当前段落 Diff 对比"
        @click="toggleDiffReview"
      >
        {{ activeDiffId ? '收起 Diff' : '审阅 Diff' }}
      </button>
      <button
        class="crb-reject-btn"
        title="全部退回本轮修改"
        @click="rejectAllChanges"
      >
        <RotateCcw :size="12" :stroke-width="2" />
        <span>全部退回</span>
      </button>
      <button class="crb-done-btn" title="全部接受本轮修改" @click="acceptAllChanges">
        <Check :size="13" :stroke-width="2.1" />
        <span>全部接受</span>
      </button>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import {
  Bold,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
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
  RotateCcw,
  Search,
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
  (e: 'comment-sent'): void
}>()

const store = useAppStore()
const scrollEl = ref<HTMLElement | null>(null)
const hostWrapEl = ref<HTMLElement | null>(null)
const muyaContainerEl = ref<HTMLElement | null>(null)
const streamPreviewEl = ref<HTMLElement | null>(null)
const floatInputEl = ref<HTMLTextAreaElement | null>(null)
const findInputEl = ref<HTMLInputElement | null>(null)

const findBar = reactive({
  open: false,
  query: '',
  index: -1,
  total: 0
})

const menuOpen = ref(false)
const saveHint = ref('')
let saveDebounceTimer: ReturnType<typeof setTimeout> | null = null
let saveHintTimer: ReturnType<typeof setTimeout> | null = null
let pendingSave: { rootPath: string; docRelPath: string; markdown: string } | null = null
let muyaInstance: Muya | null = null
let suppressMuyaChange = false
let pluginsRegistered = false
const docHistoryCache = new Map<
  string,
  { history: ReturnType<Muya['getHistory']>; markdown: string }
>()
const docScrollMap = new Map<string, number>()
let lastLoadedDocKey: string | null = null

function getDocCacheKey(rootPath?: string | null, docRelPath?: string | null): string | null {
  if (!rootPath || !docRelPath) return null
  return `${rootPath}::${docRelPath}`
}

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
const scrollPaneWidth = ref(1000)
const activeMarginThreadId = ref<string | null>(null)
const marginReplyDraft = ref('')

const activeEngineDisplayName = computed(() => {
  const eng = store.activeSession?.engine || store.preferredEngine || 'agy'
  if (eng === 'claude') return 'Claude'
  if (eng === 'codex') return 'Codex'
  return 'AGY'
})

const openThreadsCount = computed(() => store.threads.filter((t) => !t.resolved).length)
const showCommentRail = computed(
  () => openThreadsCount.value > 0 && scrollPaneWidth.value >= 560
)
const isCompactCommentRail = computed(
  () => scrollPaneWidth.value >= 560 && scrollPaneWidth.value < 860
)

function resolveThreadDomElement(
  thread: Pick<CommentThread, 'blockIndex' | 'quote'>,
  els?: HTMLElement[],
  totalMdBlocks?: number
): HTMLElement | null {
  const list = els ?? getTopBlockElements()
  if (list.length === 0) return null
  const total = totalMdBlocks ?? Math.max(list.length, splitTopBlocks(store.markdown).length)
  const domIdx = thread.quote
    ? findDomBlockIndex(thread.quote, thread.blockIndex, list, total)
    : -1
  return domIdx >= 0
    ? list[domIdx]
    : list[Math.min(Math.max(0, thread.blockIndex), Math.max(0, list.length - 1))] || null
}

const marginThreadGroups = computed(() => {
  void marginTick.value
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  const totalMdBlocks = Math.max(els.length, splitTopBlocks(store.markdown).length)
  const openThreads = store.threads.filter((t) => !t.resolved)
  const raw = openThreads.map((thread, idx) => {
    const el = resolveThreadDomElement(thread, els, totalMdBlocks)
    let top = 24 + idx * 44
    if (wrap && el) {
      top = Math.max(12, el.getBoundingClientRect().top - wrap.getBoundingClientRect().top + 2)
    }
    const estHeight = showCommentRail.value
      ? Math.min(
          380,
          Math.max(
            92,
            28 +
              thread.messages.reduce((acc, m) => {
                const clamped = isLongComment(m.text) && !expandedCommentMsgs[m.id]
                const h = clamped ? 136 : Math.min(260, 30 + Math.ceil(m.text.length / 18) * 19)
                return acc + h
              }, 0) +
              (activeMarginThreadId.value === thread.id ? 78 : 0)
          )
        )
      : 32
    return { thread, top, estHeight }
  })
  // Prevent vertical overlap when multiple comments anchor near each other
  raw.sort((a, b) => a.top - b.top)
  let nextMinTop = 0
  return raw.map((item) => {
    const adjusted = item.top < nextMinTop ? nextMinTop : item.top
    nextMinTop = adjusted + item.estHeight + 14
    return { thread: item.thread, top: adjusted }
  })
})

function formatCommentTime(ts: number): string {
  const d = new Date(ts)
  const mon = d.getMonth() + 1
  const day = d.getDate()
  const hh = String(d.getHours()).padStart(2, '0')
  const mm = String(d.getMinutes()).padStart(2, '0')
  return `${mon}月${day}日 ${hh}:${mm}`
}

const commentMd = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
})
const expandedCommentMsgs = reactive<Record<string, boolean>>({})

function isLongComment(text: string): boolean {
  return text.length > 150 || text.split(/\r?\n/).length > 5
}

function toggleCommentExpand(msgId: string) {
  expandedCommentMsgs[msgId] = !expandedCommentMsgs[msgId]
  nextTick(() => {
    marginTick.value += 1
  })
}

function renderStreamMd(mdText: string): string {
  const html = commentMd.render(mdText || '')
  const safe = DOMPurify.sanitize(html)
  // Inject inline blinking agent caret right before the last closing block tag so it flows naturally with formatted text
  const caretHtml = '<span class="agent-caret"></span>'
  const idx = safe.lastIndexOf('</')
  if (idx > 0) {
    return safe.slice(0, idx) + caretHtml + safe.slice(idx)
  }
  return safe + caretHtml
}

function renderCommentMd(text: string): string {
  const html = commentMd.render(text || '')
  const withMentions = html.replace(
    /(@(?:Claude|AGY|Codex|AI)\b)/gi,
    '<span class="mtc-mention">$1</span>'
  )
  return DOMPurify.sanitize(withMentions)
}

function selectMarginThread(thread: CommentThread) {
  if (activeMarginThreadId.value !== thread.id) {
    activeMarginThreadId.value = thread.id
    marginReplyDraft.value = ''
  }
  previewCommentAnchor(thread)
}

function previewCommentAnchor(thread: Pick<CommentThread, 'blockIndex' | 'quote'>) {
  const wrap = hostWrapEl.value
  const target = resolveThreadDomElement(thread)
  if (!wrap || !target) return
  const wRect = wrap.getBoundingClientRect()
  const tRect = target.getBoundingClientRect()
  reviewFocusBox.top = Math.max(0, tRect.top - wRect.top - 4)
  reviewFocusBox.left = Math.max(0, tRect.left - wRect.left - 8)
  reviewFocusBox.width = tRect.width + 16
  reviewFocusBox.height = Math.max(28, tRect.height + 8)
  reviewFocusBox.visible = true
}

function clearCommentAnchorPreview() {
  if (reviewFocusIndex.value < 0) {
    reviewFocusBox.visible = false
  }
}

async function submitMarginReply(threadId: string) {
  const text = marginReplyDraft.value.trim()
  if (!text) return
  marginReplyDraft.value = ''
  await store.replyInThread(threadId, text)
  nextTick(() => {
    marginTick.value += 1
  })
}

async function resolveMarginThread(threadId: string) {
  activeMarginThreadId.value = null
  reviewFocusBox.visible = false
  await store.resolveThread(threadId)
  nextTick(() => {
    marginTick.value += 1
  })
}

const animState = reactive({
  runId: 0,
  running: false,
  visible: false,
  paused: false,
  skip: false,
  skipAll: false,
  pinned: false,
  activeBlockIndex: -1,
  batchCurrent: 0,
  batchTotal: 0,
  phase: 'lock' as 'lock' | 'writing' | 'settled' | 'fading',
  progress: 0,
  label: '',
  badge: '',
  detail: '',
  streamText: '',
  streamHtml: '',
  showStream: false,
  boxTop: 0,
  boxLeft: 0,
  boxWidth: 0,
  boxHeight: 0,
  cursorX: 0,
  cursorY: 0
})

// Persistent human-certainty per-turn Diff review (Form A: gutter bars + floating word-level Diff + Accept/Reject)
interface PendingBlockChange {
  id: string
  blockIndex: number
  type: 'replace_block' | 'insert_block' | 'delete_block'
  oldMarkdown: string
  newMarkdown: string
  label: string
}

const pendingChanges = ref<PendingBlockChange[]>([])
const reviewPos = ref(0)
const reviewFocusIndex = ref(-1)
const activeDiffId = ref<string | null>(null)
const diffCardPos = reactive({ top: 0, left: 0, width: 640 })
const reviewFocusBox = reactive({
  visible: false,
  top: 0,
  left: 0,
  width: 0,
  height: 0
})
let reviewFocusTimer: ReturnType<typeof setTimeout> | null = null

const activeDiffChange = computed(
  () => pendingChanges.value.find((c) => c.id === activeDiffId.value) ?? null
)

function diffKindLabel(type: PendingBlockChange['type']): string {
  if (type === 'insert_block') return '新增'
  if (type === 'delete_block') return '删除'
  return '修改'
}

function tokenizeForDiff(text: string): string[] {
  const clean = text.replace(/^#{1,6}\s+/gm, '').trim()
  return clean.match(/[\u4e00-\u9fff]|[A-Za-z0-9_]+|\s+|[^\s\u4e00-\u9fffA-Za-z0-9_]+/g) || []
}

function computeInlineDiff(
  oldText: string,
  newText: string
): Array<{ kind: 'same' | 'del' | 'add'; text: string }> {
  const a = tokenizeForDiff(oldText)
  const b = tokenizeForDiff(newText)
  if (a.length === 0) return [{ kind: 'add', text: b.join('') }]
  if (b.length === 0) return [{ kind: 'del', text: a.join('') }]

  // Guard against extreme token counts for < 2ms responsiveness
  if (a.length * b.length > 160000) {
    return [
      { kind: 'del', text: a.join('') },
      { kind: 'add', text: b.join('') }
    ]
  }

  const m = a.length
  const n = b.length
  const dp: Uint16Array[] = Array.from({ length: m + 1 }, () => new Uint16Array(n + 1))
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      if (a[i] === b[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1
      } else {
        dp[i][j] = dp[i + 1][j] >= dp[i][j + 1] ? dp[i + 1][j] : dp[i][j + 1]
      }
    }
  }

  const raw: Array<{ kind: 'same' | 'del' | 'add'; text: string }> = []
  let i = 0
  let j = 0
  while (i < m && j < n) {
    if (a[i] === b[j]) {
      raw.push({ kind: 'same', text: a[i] })
      i++
      j++
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      raw.push({ kind: 'del', text: a[i] })
      i++
    } else {
      raw.push({ kind: 'add', text: b[j] })
      j++
    }
  }
  while (i < m) raw.push({ kind: 'del', text: a[i++] })
  while (j < n) raw.push({ kind: 'add', text: b[j++] })

  const merged: Array<{ kind: 'same' | 'del' | 'add'; text: string }> = []
  for (const seg of raw) {
    const prev = merged[merged.length - 1]
    if (prev && prev.kind === seg.kind) {
      prev.text += seg.text
    } else {
      merged.push({ ...seg })
    }
  }
  return merged
}

const activeDiffSegments = computed(() => {
  const ch = activeDiffChange.value
  if (!ch) return []
  return computeInlineDiff(ch.oldMarkdown, ch.newMarkdown)
})

const changedBarRects = computed(() => {
  void marginTick.value
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  if (!wrap || els.length === 0 || pendingChanges.value.length === 0) return []
  const wRect = wrap.getBoundingClientRect()
  const totalMdBlocks = Math.max(els.length, splitTopBlocks(store.markdown).length)
  const out: Array<{ changeId: string; blockIndex: number; top: number; left: number; height: number }> = []
  for (const ch of pendingChanges.value) {
    const domIdx = findDomBlockIndex(
      ch.newMarkdown || ch.oldMarkdown,
      ch.blockIndex,
      els,
      totalMdBlocks
    )
    const el = domIdx >= 0 ? els[domIdx] : els[Math.min(Math.max(0, ch.blockIndex), els.length - 1)]
    if (!el) continue
    const tRect = el.getBoundingClientRect()
    out.push({
      changeId: ch.id,
      blockIndex: ch.blockIndex,
      top: Math.max(0, tRect.top - wRect.top + 2),
      left: Math.max(0, tRect.left - wRect.left - 12),
      height: Math.max(18, tRect.height - 4)
    })
  }
  return out
})

function recordPendingChange(
  type: PendingBlockChange['type'],
  blockIndex: number,
  oldMarkdown: string,
  newMarkdown: string,
  label: string
) {
  if (blockIndex < 0) return

  if (type === 'delete_block') {
    for (const c of pendingChanges.value) {
      if (c.blockIndex > blockIndex) c.blockIndex -= 1
    }
    pendingChanges.value = [
      ...pendingChanges.value,
      {
        id: `chg-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`,
        blockIndex,
        type,
        oldMarkdown,
        newMarkdown: '',
        label
      }
    ].sort((a, b) => a.blockIndex - b.blockIndex)
    marginTick.value += 1
    return
  }

  // Split multi-paragraph inserts/replacements (e.g. inserting an entire chapter at once!)
  // so EVERY newly created top-level block gets its own gutter bar & Diff review entry.
  const subBlocks = splitTopBlocks(newMarkdown)
  const parts = subBlocks.length > 0 ? subBlocks : [newMarkdown]

  if (type === 'insert_block') {
    const shift = parts.length
    for (const c of pendingChanges.value) {
      if (c.blockIndex >= blockIndex) c.blockIndex += shift
    }
    const added: PendingBlockChange[] = parts.map((partMd, offset) => ({
      id: `chg-${Date.now().toString(36)}-${offset}-${Math.random().toString(36).slice(2, 5)}`,
      blockIndex: blockIndex + offset,
      type: 'insert_block',
      oldMarkdown: '',
      newMarkdown: partMd,
      label
    }))
    pendingChanges.value = [...pendingChanges.value, ...added].sort(
      (a, b) => a.blockIndex - b.blockIndex
    )
    marginTick.value += 1
    return
  }

  // replace_block: first block replaces oldMarkdown; any extra blocks (parts.length > 1) are tracked as insert_block
  const extraCount = parts.length - 1
  if (extraCount > 0) {
    for (const c of pendingChanges.value) {
      if (c.blockIndex > blockIndex) c.blockIndex += extraCount
    }
  }

  const existing = pendingChanges.value.find(
    (c) => c.blockIndex === blockIndex && c.type !== 'delete_block'
  )
  const additions: PendingBlockChange[] = []
  if (existing) {
    // Preserve true pre-turn oldMarkdown if same block edited twice in one turn
    existing.newMarkdown = parts[0]
    existing.label = label
  } else {
    additions.push({
      id: `chg-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`,
      blockIndex,
      type: 'replace_block',
      oldMarkdown,
      newMarkdown: parts[0],
      label
    })
  }
  for (let offset = 1; offset < parts.length; offset++) {
    additions.push({
      id: `chg-${Date.now().toString(36)}-${offset}-${Math.random().toString(36).slice(2, 5)}`,
      blockIndex: blockIndex + offset,
      type: 'insert_block',
      oldMarkdown: '',
      newMarkdown: parts[offset],
      label
    })
  }
  if (additions.length > 0) {
    pendingChanges.value = [...pendingChanges.value, ...additions].sort(
      (a, b) => a.blockIndex - b.blockIndex
    )
  }
  marginTick.value += 1
}

function resolveChangeDomElement(ch: {
  blockIndex: number
  newMarkdown?: string
  oldMarkdown?: string
}): HTMLElement | null {
  const els = getTopBlockElements()
  if (els.length === 0) return null
  const totalMdBlocks = Math.max(els.length, splitTopBlocks(store.markdown).length)
  const domIdx = findDomBlockIndex(
    ch.newMarkdown || ch.oldMarkdown,
    ch.blockIndex,
    els,
    totalMdBlocks
  )
  return domIdx >= 0
    ? els[domIdx]
    : els[Math.min(Math.max(0, ch.blockIndex), Math.max(0, els.length - 1))] || null
}

function resolveChangeMdBlockIndex(
  blocks: string[],
  ch: { blockIndex: number; type: 'replace_block' | 'insert_block' | 'delete_block'; newMarkdown: string }
): number {
  if (ch.type === 'delete_block') {
    return Math.min(blocks.length, Math.max(0, ch.blockIndex))
  }
  const needle = ch.newMarkdown.trim()
  if (needle) {
    const exactMatches: number[] = []
    for (let i = 0; i < blocks.length; i++) {
      if (blocks[i].trim() === needle) exactMatches.push(i)
    }
    if (exactMatches.length > 0) {
      return exactMatches.reduce((best, cur) =>
        Math.abs(cur - ch.blockIndex) < Math.abs(best - ch.blockIndex) ? cur : best
      )
    }
    const norm = (s: string) => s.replace(/\s+/g, ' ').trim()
    const needleNorm = norm(needle)
    const fuzzyMatches: number[] = []
    for (let i = 0; i < blocks.length; i++) {
      if (norm(blocks[i]) === needleNorm) fuzzyMatches.push(i)
    }
    if (fuzzyMatches.length > 0) {
      return fuzzyMatches.reduce((best, cur) =>
        Math.abs(cur - ch.blockIndex) < Math.abs(best - ch.blockIndex) ? cur : best
      )
    }
  }
  return ch.blockIndex
}

function positionDiffCard(ch: { blockIndex: number; newMarkdown?: string; oldMarkdown?: string }) {
  const wrap = hostWrapEl.value
  const target = resolveChangeDomElement(ch)
  if (!wrap || !target) return
  const wRect = wrap.getBoundingClientRect()
  const tRect = target.getBoundingClientRect()
  const cardWidth = Math.min(Math.max(360, tRect.width + 20), wrap.clientWidth)
  diffCardPos.left = Math.max(0, tRect.left - wRect.left - 10)
  diffCardPos.width = cardWidth
  // Anchor neatly below the focused paragraph so it never collides with headings above
  diffCardPos.top = Math.max(8, tRect.bottom - wRect.top + 8)
}

function openDiffForChange(changeId: string) {
  const idx = pendingChanges.value.findIndex((c) => c.id === changeId)
  if (idx < 0) return
  const ch = pendingChanges.value[idx]
  reviewPos.value = idx
  activeDiffId.value = ch.id
  focusChangedBlock(ch)
  nextTick(() => positionDiffCard(ch))
}

function toggleDiffReview() {
  if (activeDiffId.value) {
    activeDiffId.value = null
    return
  }
  if (pendingChanges.value.length === 0) return
  const safePos = Math.min(reviewPos.value, pendingChanges.value.length - 1)
  openDiffForChange(pendingChanges.value[safePos].id)
}

function focusChangedBlock(ch: { blockIndex: number; newMarkdown?: string; oldMarkdown?: string }) {
  const wrap = hostWrapEl.value
  const target = resolveChangeDomElement(ch)
  if (!wrap || !target) return
  reviewFocusIndex.value = ch.blockIndex
  userScrollPausedUntil = 0
  scrollBlockToGoldenZone(target)
  const wRect = wrap.getBoundingClientRect()
  const tRect = target.getBoundingClientRect()
  reviewFocusBox.top = Math.max(0, tRect.top - wRect.top - 5)
  reviewFocusBox.left = Math.max(0, tRect.left - wRect.left - 10)
  reviewFocusBox.width = tRect.width + 20
  reviewFocusBox.height = Math.max(30, tRect.height + 10)
  reviewFocusBox.visible = true
  if (reviewFocusTimer) clearTimeout(reviewFocusTimer)
  reviewFocusTimer = setTimeout(() => {
    reviewFocusBox.visible = false
    reviewFocusIndex.value = -1
  }, 1500)
}

function stepChangedBlock(delta: number) {
  const len = pendingChanges.value.length
  if (len === 0) return
  reviewPos.value = (reviewPos.value + delta + len) % len
  openDiffForChange(pendingChanges.value[reviewPos.value].id)
}

function acceptChange(changeId: string) {
  const idx = pendingChanges.value.findIndex((c) => c.id === changeId)
  if (idx < 0) return
  pendingChanges.value = pendingChanges.value.filter((c) => c.id !== changeId)
  if (pendingChanges.value.length === 0) {
    clearChangedReview()
    store.showToast('本轮修改已全部接受')
    return
  }
  const nextIdx = Math.min(idx, pendingChanges.value.length - 1)
  reviewPos.value = nextIdx
  if (activeDiffId.value === changeId) {
    openDiffForChange(pendingChanges.value[nextIdx].id)
  }
  marginTick.value += 1
}

async function rejectChange(changeId: string) {
  const idx = pendingChanges.value.findIndex((c) => c.id === changeId)
  if (idx < 0) return
  const ch = pendingChanges.value[idx]
  const currentMd = store.markdown || (muyaInstance ? muyaInstance.getMarkdown() : '')
  const blocks = splitTopBlocks(currentMd)
  const targetIdx = resolveChangeMdBlockIndex(blocks, ch)

  if (ch.type === 'replace_block') {
    if (targetIdx >= 0 && targetIdx < blocks.length) {
      blocks[targetIdx] = ch.oldMarkdown
    }
  } else if (ch.type === 'insert_block') {
    if (targetIdx >= 0 && targetIdx < blocks.length) {
      blocks.splice(targetIdx, 1)
    }
    for (const other of pendingChanges.value) {
      if (other.id !== ch.id && other.blockIndex > targetIdx) {
        other.blockIndex -= 1
      }
    }
  } else if (ch.type === 'delete_block') {
    const insertAt = Math.min(blocks.length, Math.max(0, targetIdx))
    blocks.splice(insertAt, 0, ch.oldMarkdown)
    for (const other of pendingChanges.value) {
      if (other.id !== ch.id && other.blockIndex >= insertAt) {
        other.blockIndex += 1
      }
    }
  }

  const nextMd = blocks.join('\n\n') + '\n'
  store.markdown = nextMd
  setMuyaMarkdownQuietly(nextMd, true)
  await store.onEditorMarkdownChanged(nextMd, true)

  pendingChanges.value = pendingChanges.value.filter((c) => c.id !== changeId)
  store.showToast(`已退回第 ${targetIdx + 1} 段原文`)
  await nextTick()
  marginTick.value += 1

  if (pendingChanges.value.length === 0) {
    clearChangedReview()
    return
  }
  const nextIdx = Math.min(idx, pendingChanges.value.length - 1)
  reviewPos.value = nextIdx
  if (activeDiffId.value === changeId) {
    openDiffForChange(pendingChanges.value[nextIdx].id)
  }
}

function acceptAllChanges() {
  clearChangedReview()
  store.showToast('已全部接受本轮修改')
}

async function rejectAllChanges() {
  if (pendingChanges.value.length === 0) return
  const currentMd = store.markdown || (muyaInstance ? muyaInstance.getMarkdown() : '')
  const blocks = splitTopBlocks(currentMd)
  // Revert from bottom to top using content-anchored indices
  const resolved = pendingChanges.value
    .map((ch) => ({ ch, targetIdx: resolveChangeMdBlockIndex(blocks, ch) }))
    .sort((a, b) => b.targetIdx - a.targetIdx)
  for (const { ch, targetIdx } of resolved) {
    if (ch.type === 'replace_block') {
      if (targetIdx >= 0 && targetIdx < blocks.length) {
        blocks[targetIdx] = ch.oldMarkdown
      }
    } else if (ch.type === 'insert_block') {
      if (targetIdx >= 0 && targetIdx < blocks.length) {
        blocks.splice(targetIdx, 1)
      }
    } else if (ch.type === 'delete_block') {
      const insertAt = Math.min(blocks.length, Math.max(0, targetIdx))
      blocks.splice(insertAt, 0, ch.oldMarkdown)
    }
  }
  const nextMd = blocks.join('\n\n') + '\n'
  store.markdown = nextMd
  setMuyaMarkdownQuietly(nextMd, true)
  await store.onEditorMarkdownChanged(nextMd, true)
  clearChangedReview()
  store.showToast('已全部退回本轮修改')
}

function clearChangedReview() {
  pendingChanges.value = []
  reviewPos.value = 0
  activeDiffId.value = null
  reviewFocusIndex.value = -1
  reviewFocusBox.visible = false
}

watch(
  () => store.activeDocRelPath,
  () => {
    clearChangedReview()
  }
)

watch(
  () => store.agentBusy,
  (busy, prev) => {
    if (busy && !prev) {
      clearChangedReview()
    }
  }
)

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
    const targetRoot = store.activeWorkspace?.rootPath
    const targetRel = store.activeDocRelPath
    if (!targetRoot || !targetRel) return
    const md = muyaInstance.getMarkdown()
    // Sync in-memory live doc immediately, debounce disk write (300ms) bound to the edited doc
    void store.onEditorMarkdownChanged(md, false, {
      rootPath: targetRoot,
      docRelPath: targetRel
    })
    pendingSave = { rootPath: targetRoot, docRelPath: targetRel, markdown: md }
    saveHint.value = '保存中...'
    if (saveDebounceTimer) clearTimeout(saveDebounceTimer)
    saveDebounceTimer = setTimeout(() => {
      void flushPendingSave()
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

function setMuyaMarkdownQuietly(md: string, preserveUndo = false) {
  if (!muyaInstance) {
    mountMuya(md)
    return
  }
  suppressMuyaChange = true
  try {
    if (preserveUndo) {
      try {
        muyaInstance.replaceContent(md)
      } catch {
        muyaInstance.setContent(md)
      }
    } else {
      muyaInstance.setContent(md)
    }
  } finally {
    suppressMuyaChange = false
  }
  refreshToc()
  marginTick.value += 1
}

const MAX_DOC_CACHE_ENTRIES = 30

function touchCacheEntry<T>(map: Map<string, T>, key: string, value: T) {
  if (map.has(key)) map.delete(key)
  map.set(key, value)
  while (map.size > MAX_DOC_CACHE_ENTRIES) {
    const oldest = map.keys().next().value
    if (oldest !== undefined) map.delete(oldest)
    else break
  }
}

function saveCurrentDocHistory() {
  if (scrollEl.value && lastLoadedDocKey) {
    touchCacheEntry(docScrollMap, lastLoadedDocKey, scrollEl.value.scrollTop)
  }
  if (!muyaInstance || !lastLoadedDocKey) return
  try {
    muyaInstance.flush()
    touchCacheEntry(docHistoryCache, lastLoadedDocKey, {
      history: muyaInstance.getHistory(),
      markdown: muyaInstance.getMarkdown()
    })
  } catch {
    // ignore if editor is torn down
  }
}

watch(
  () => store.activeWorkspace?.rootPath,
  (nextRoot, prevRoot) => {
    if (prevRoot && nextRoot !== prevRoot) {
      docHistoryCache.clear()
      docScrollMap.clear()
      lastLoadedDocKey = null
    }
  }
)

function restoreDocScroll(docKey: string | null) {
  const targetTop = docKey ? docScrollMap.get(docKey) ?? 0 : 0
  requestAnimationFrame(() => {
    if (scrollEl.value) {
      scrollEl.value.scrollTop = targetTop
    }
  })
}

watch(
  () => [store.activeDocRelPath, store.externalDocSeq] as const,
  async ([newPath]) => {
    abortAnimation()
    selToolbar.open = false
    floatComment.open = false
    threadPop.threadId = null
    const newKey = getDocCacheKey(store.activeWorkspace?.rootPath, newPath)
    const isSameDoc = Boolean(newKey && newKey === lastLoadedDocKey)
    if (!isSameDoc) {
      saveCurrentDocHistory()
      if (findBar.open) {
        closeFindBar()
      }
    }
    if (!newPath) {
      lastLoadedDocKey = null
      return
    }
    if (!/\.(md|markdown)$/i.test(newPath)) {
      lastLoadedDocKey = newKey
      store.toc = []
      return
    }
    await nextTick()
    if (!muyaInstance) {
      mountMuya(store.markdown)
      lastLoadedDocKey = newKey
      const mounted = muyaInstance as Muya | null
      const cached = newKey ? docHistoryCache.get(newKey) : undefined
      if (mounted && cached && cached.markdown === mounted.getMarkdown()) {
        try {
          mounted.setHistory(cached.history)
        } catch {}
      }
      restoreDocScroll(newKey)
    } else if (isSameDoc) {
      setMuyaMarkdownQuietly(store.markdown, true)
    } else {
      setMuyaMarkdownQuietly(store.markdown, false)
      lastLoadedDocKey = newKey
      const cached = newKey ? docHistoryCache.get(newKey) : undefined
      if (cached && cached.markdown === muyaInstance.getMarkdown()) {
        try {
          muyaInstance.setHistory(cached.history)
        } catch {}
      }
      restoreDocScroll(newKey)
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
const waitForLayoutFrame = () =>
  new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())))

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

function normalizeTextForBlockMatch(raw: string): string {
  return raw
    .replace(/^---[\s\S]*?---$/m, (m) => m.replace(/^---|---$/gm, ''))
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/^>\s?/gm, '')
    .replace(/^(\s*[-*+]|\s*\d+\.)\s+/gm, '')
    .replace(/[`*_~]/g, '')
    .replace(/\s+/g, '')
    .trim()
    .toLowerCase()
}

function findDomBlockIndex(
  mdText: string | undefined,
  fallbackBlockIdx: number,
  els: HTMLElement[],
  totalMdBlocks: number
): number {
  if (els.length === 0) return -1
  const needle = mdText ? normalizeTextForBlockMatch(mdText) : ''
  const expectedRatio =
    totalMdBlocks > 1 ? Math.min(1, Math.max(0, fallbackBlockIdx / (totalMdBlocks - 1))) : 0

  if (needle.length >= 2) {
    let bestIdx = -1
    let bestScore = -1
    let bestDist = Infinity
    for (let i = 0; i < els.length; i++) {
      const hay = normalizeTextForBlockMatch(els[i].innerText || '')
      if (!hay) continue
      let score = 0
      if (hay === needle) {
        score = 100
      } else if (hay.includes(needle) || needle.includes(hay)) {
        score = 80
      } else {
        const prefix = needle.slice(0, Math.min(16, needle.length))
        if (prefix.length >= 6 && hay.includes(prefix)) {
          score = 60
        }
      }
      if (score > 0) {
        const ratio = els.length > 1 ? i / (els.length - 1) : 0
        const dist = Math.abs(ratio - expectedRatio)
        if (score > bestScore || (score === bestScore && dist < bestDist)) {
          bestScore = score
          bestDist = dist
          bestIdx = i
        }
      }
    }
    if (bestIdx >= 0) return bestIdx
  }

  const scaled =
    totalMdBlocks > 0
      ? Math.round((fallbackBlockIdx / Math.max(1, totalMdBlocks)) * els.length)
      : fallbackBlockIdx
  return Math.min(Math.max(0, scaled), els.length - 1)
}

let activeInsertSpacerEl: HTMLElement | null = null
function setInsertSpacer(el: HTMLElement | null, extraPx: number) {
  if (activeInsertSpacerEl && activeInsertSpacerEl !== el) {
    activeInsertSpacerEl.style.marginBottom = ''
    activeInsertSpacerEl.style.transition = ''
    activeInsertSpacerEl = null
  }
  if (!el) return
  el.style.transition = 'margin-bottom 0.14s ease'
  el.style.marginBottom = `${Math.max(0, Math.round(extraPx))}px`
  activeInsertSpacerEl = el
}
function clearInsertSpacer() {
  if (activeInsertSpacerEl) {
    activeInsertSpacerEl.style.marginBottom = ''
    activeInsertSpacerEl.style.transition = ''
    activeInsertSpacerEl = null
  }
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
  shouldScroll = true,
  matchMarkdown?: string,
  totalMdBlocks?: number
) {
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  const domIdx =
    matchMarkdown !== undefined
      ? findDomBlockIndex(matchMarkdown, blockIndex, els, totalMdBlocks ?? els.length)
      : Math.min(Math.max(0, blockIndex), Math.max(0, els.length - 1))
  const target = els[domIdx]
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
  animState.cursorX = Math.max(animState.boxLeft + 8, animState.boxLeft + animState.boxWidth - 246)
  animState.cursorY = Math.max(0, animState.boxTop - 26)
  animState.label = label
  animState.detail = detail
  animState.visible = true
}

function positionOverlayForInsert(
  insertIndex: number,
  label: string,
  detail: string,
  shouldScroll = true,
  prevBlockMarkdown?: string,
  totalMdBlocks?: number
) {
  const wrap = hostWrapEl.value
  const els = getTopBlockElements()
  if (!wrap) return

  clearInsertSpacer()
  const wRect = wrap.getBoundingClientRect()
  if (els.length === 0) {
    animState.boxTop = 8
    animState.boxLeft = 2
    animState.boxWidth = Math.max(240, wrap.clientWidth - 4)
    animState.boxHeight = 44
  } else if (insertIndex <= 0) {
    const first = els[0]
    if (shouldScroll) scrollBlockToGoldenZone(first)
    const fRect = first.getBoundingClientRect()
    animState.boxTop = Math.max(0, fRect.top - wRect.top - 6)
    animState.boxLeft = Math.max(0, fRect.left - wRect.left - 10)
    animState.boxWidth = fRect.width + 20
    animState.boxHeight = 44
  } else {
    const prevDomIdx =
      prevBlockMarkdown !== undefined
        ? findDomBlockIndex(prevBlockMarkdown, insertIndex - 1, els, totalMdBlocks ?? els.length)
        : Math.min(insertIndex - 1, els.length - 1)
    const prev = els[Math.max(0, prevDomIdx)]
    if (shouldScroll) scrollBlockToGoldenZone(prev)
    const pRect = prev.getBoundingClientRect()
    animState.boxTop = Math.max(0, pRect.bottom - wRect.top + 8)
    animState.boxLeft = Math.max(0, pRect.left - wRect.left - 10)
    animState.boxWidth = pRect.width + 20
    animState.boxHeight = 44
    // When inserting in the middle of the document, open space below prev so the typing box never overlaps the next block
    if (prevDomIdx >= 0 && prevDomIdx < els.length - 1) {
      setInsertSpacer(prev, animState.boxHeight + 16)
    }
  }

  animState.cursorX = Math.max(animState.boxLeft + 8, animState.boxLeft + animState.boxWidth - 246)
  animState.cursorY = Math.max(0, animState.boxTop - 26)
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

function skipCurrent() {
  animState.skip = true
}

function skipAllOps() {
  animState.skip = true
  animState.skipAll = true
}

function abortAnimation(persistHumanContent = false) {
  const wasRunning = animState.running
  clearInsertSpacer()
  animState.runId += 1
  animState.running = false
  animState.visible = false
  animState.pinned = false
  animState.skipAll = false
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

const PUNCT_PAUSE_RE = /[，。；：！？、,.;!?]/

interface BlockDiffUnit {
  kind: 'modify' | 'insert' | 'delete'
  unitIdx: number
  oldMd: string
  newMd: string
  anchorPrevMd: string
}

function escapeAnimHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/**
 * Layer 1: Block-level LCS Diff between curBlocks and nextBlocks.
 * Produces minimal ordered edit units (modify / insert / delete) with exact running indices
 * and content-anchored previous block text so we never drift on loose lists or front-matter.
 */
function computeBlockDiffPlan(curBlocks: string[], nextBlocks: string[]): BlockDiffUnit[] {
  const m = curBlocks.length
  const n = nextBlocks.length
  const dp: Uint16Array[] = Array.from({ length: m + 1 }, () => new Uint16Array(n + 1))
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      if (curBlocks[i] === nextBlocks[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1
      } else {
        dp[i][j] = dp[i + 1][j] >= dp[i][j + 1] ? dp[i + 1][j] : dp[i][j + 1]
      }
    }
  }

  type RawStep = { kind: 'equal' | 'delete' | 'insert'; text: string }
  const raw: RawStep[] = []
  let i = 0
  let j = 0
  while (i < m && j < n) {
    if (curBlocks[i] === nextBlocks[j]) {
      raw.push({ kind: 'equal', text: curBlocks[i] })
      i++
      j++
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      raw.push({ kind: 'delete', text: curBlocks[i] })
      i++
    } else {
      raw.push({ kind: 'insert', text: nextBlocks[j] })
      j++
    }
  }
  while (i < m) raw.push({ kind: 'delete', text: curBlocks[i++] })
  while (j < n) raw.push({ kind: 'insert', text: nextBlocks[j++] })

  // Pair adjacent delete + insert runs into modify units, and compute running indices
  const units: BlockDiffUnit[] = []
  let runIdx = 0
  let lastCommittedMd = ''
  let k = 0
  while (k < raw.length) {
    if (raw[k].kind === 'equal') {
      lastCommittedMd = raw[k].text
      runIdx += 1
      k += 1
      continue
    }
    const dels: string[] = []
    const ins: string[] = []
    while (k < raw.length && raw[k].kind !== 'equal') {
      if (raw[k].kind === 'delete') dels.push(raw[k].text)
      else ins.push(raw[k].text)
      k += 1
    }
    const pairCount = Math.min(dels.length, ins.length)
    for (let p = 0; p < pairCount; p++) {
      units.push({
        kind: 'modify',
        unitIdx: runIdx,
        oldMd: dels[p],
        newMd: ins[p],
        anchorPrevMd: lastCommittedMd
      })
      lastCommittedMd = ins[p]
      runIdx += 1
    }
    for (let p = pairCount; p < dels.length; p++) {
      units.push({
        kind: 'delete',
        unitIdx: runIdx,
        oldMd: dels[p],
        newMd: '',
        anchorPrevMd: lastCommittedMd
      })
      // runIdx does not advance on delete
    }
    for (let p = pairCount; p < ins.length; p++) {
      units.push({
        kind: 'insert',
        unitIdx: runIdx,
        oldMd: '',
        newMd: ins[p],
        anchorPrevMd: lastCommittedMd
      })
      lastCommittedMd = ins[p]
      runIdx += 1
    }
  }
  return units
}

interface InlineEditHunk {
  kind: 'same' | 'edit'
  sameText?: string
  delText?: string
  addText?: string
}

function groupInlineDiffIntoHunks(
  segments: Array<{ kind: 'same' | 'del' | 'add'; text: string }>
): InlineEditHunk[] {
  const hunks: InlineEditHunk[] = []
  let i = 0
  while (i < segments.length) {
    if (segments[i].kind === 'same') {
      hunks.push({ kind: 'same', sameText: segments[i].text })
      i++
      continue
    }
    let delText = ''
    let addText = ''
    while (i < segments.length && segments[i].kind !== 'same') {
      if (segments[i].kind === 'del') delText += segments[i].text
      else addText += segments[i].text
      i++
    }
    hunks.push({ kind: 'edit', delText, addText })
  }
  return hunks
}

/**
 * Human-like sequential inline editing renderer:
 * - Past hunks (< activeEditIdx): already finalized (addText only, delText gone!)
 * - Active hunk (=== activeEditIdx):
 *     - 'select': old text highlighted as selected + caret (just before Backspace)
 *     - 'type':   old text deleted, new text typed char-by-char at the live caret
 * - Future hunks (> activeEditIdx): still untouched original text (delText)
 * Result: when the final hunk finishes, the DOM text & line-breaks match Muya 100% -> ZERO endpoint jump!
 */
function renderHumanInlineEditHtml(
  hunks: InlineEditHunk[],
  activeEditIdx: number,
  subPhase: 'select' | 'type' | 'done',
  typedChars: number
): string {
  let html = ''
  let editSeq = 0
  for (const h of hunks) {
    if (h.kind === 'same') {
      html += escapeAnimHtml(h.sameText || '')
      continue
    }
    const myEditIdx = editSeq++
    const del = h.delText || ''
    const add = h.addText || ''

    if (myEditIdx < activeEditIdx || subPhase === 'done') {
      if (add) {
        html += `<span class="anim-diff-done">${escapeAnimHtml(add)}</span>`
      }
    } else if (myEditIdx === activeEditIdx) {
      if (subPhase === 'select') {
        if (del) {
          html += `<span class="anim-diff-select">${escapeAnimHtml(del)}</span><span class="agent-caret"></span>`
        } else {
          html += '<span class="agent-caret"></span>'
        }
      } else {
        const chars = Array.from(add)
        const shown = chars.slice(0, Math.min(chars.length, typedChars)).join('')
        if (shown) {
          html += `<span class="anim-diff-add">${escapeAnimHtml(shown)}</span><span class="agent-caret"></span>`
        } else {
          html += '<span class="agent-caret"></span>'
        }
      }
    } else {
      // Untouched future edit: keep original text in place
      if (del) {
        html += escapeAnimHtml(del)
      }
    }
  }
  return `<p style="margin:0;white-space:pre-wrap">${html}</p>`
}

async function drainOpQueue() {
  const myRunId = ++animState.runId
  animState.running = true
  animState.paused = false
  animState.skipAll = false
  animState.batchCurrent = 0
  animState.batchTotal = store.pendingDocOps.filter(
    (x) => x.op.docRelPath === store.activeDocRelPath
  ).length

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

    const remainingOpsForDoc = store.pendingDocOps.filter(
      (x) => x.op.docRelPath === store.activeDocRelPath
    ).length

    const currentMd = store.markdown || (muyaInstance ? muyaInstance.getMarkdown() : '')
    const blocks = splitTopBlocks(currentMd)

    // Determine authoritative target blocks (Invariant 1: Truth Convergence)
    let nextBlocks: string[]
    if (op.resultingMarkdown !== undefined) {
      nextBlocks = splitTopBlocks(op.resultingMarkdown)
    } else {
      nextBlocks = [...blocks]
      const targetIdx =
        op.type === 'insert_block'
          ? Math.min(blocks.length, Math.max(0, op.afterBlockIndex + 1))
          : Math.min(Math.max(0, op.blockIndex), Math.max(0, blocks.length - 1))
      if (op.type === 'delete_block') {
        nextBlocks.splice(targetIdx, 1)
      } else {
        const parts = splitTopBlocks(op.newMarkdown ?? '')
        const toApply = parts.length > 0 ? parts : [op.newMarkdown ?? '']
        if (op.type === 'insert_block') {
          nextBlocks.splice(targetIdx, 0, ...toApply)
        } else {
          nextBlocks.splice(targetIdx, 1, ...toApply)
        }
      }
    }

    // Layer 1: Compute exact block-level diff plan
    const diffUnits = computeBlockDiffPlan(blocks, nextBlocks)
    if (diffUnits.length === 0) {
      // No visual diff, ensure final markdown converges
      if (op.resultingMarkdown !== undefined && op.resultingMarkdown !== store.markdown) {
        store.markdown = op.resultingMarkdown
        setMuyaMarkdownQuietly(op.resultingMarkdown)
      }
      continue
    }

    animState.batchTotal = Math.max(
      animState.batchTotal,
      animState.batchCurrent + diffUnits.length + remainingOpsForDoc
    )

    for (let u = 0; u < diffUnits.length; u++) {
      if (animState.runId !== myRunId) return
      animState.batchCurrent += 1
      const unit = diffUnits[u]
      const unitType: PendingBlockChange['type'] =
        unit.kind === 'delete'
          ? 'delete_block'
          : unit.kind === 'insert'
            ? 'insert_block'
            : 'replace_block'
      const unitIdx = unit.unitIdx
      const unitOldMd = unit.oldMd
      const unitMd = unit.newMd
      const remainingUnits = remainingOpsForDoc + (diffUnits.length - 1 - u)

      animState.skip = animState.skipAll
      let modifyAnchorEl: HTMLElement | null = null
      let modifyOrigBoxH = 0

      if (!animState.skipAll) {
        animState.activeBlockIndex = unitIdx
        animState.phase = 'lock'
        animState.progress = 0
        animState.streamText = ''
        animState.streamHtml = ''
        animState.showStream = false
        animState.badge =
          animState.batchTotal > 1 ? `${animState.batchCurrent}/${animState.batchTotal}` : ''
        if (unitType === 'insert_block') {
          positionOverlayForInsert(
            unitIdx,
            op.label,
            `定位第 ${unitIdx + 1} 段`,
            true,
            unit.anchorPrevMd,
            blocks.length
          )
        } else {
          const els = getTopBlockElements()
          const domIdx = findDomBlockIndex(unitOldMd, unitIdx, els, blocks.length)
          modifyAnchorEl = domIdx >= 0 ? els[domIdx] : null
          positionOverlayOnBlock(
            unitIdx,
            op.label,
            `定位第 ${unitIdx + 1} 段`,
            true,
            unitOldMd,
            blocks.length
          )
          modifyOrigBoxH = animState.boxHeight
        }
        await sleep(remainingUnits >= 3 ? 130 : 240)
        while (animState.paused && animState.runId === myRunId) await sleep(80)
        if (animState.runId !== myRunId) return
      }

      // Layer 2: Minimal Diff-Level Animation vs Streaming Insert/Rewrite
      if (unitType === 'replace_block' && !animState.skip) {
        const inlineSegs = computeInlineDiff(unitOldMd, unitMd)
        const totalOld = Math.max(1, Array.from(unitOldMd).length)
        const totalNew = Math.max(1, Array.from(unitMd).length)
        const changedChars = inlineSegs
          .filter((s) => s.kind !== 'same')
          .reduce((acc, s) => acc + Array.from(s.text).length, 0)
        const changeRatio = changedChars / (totalOld + totalNew)
        const addedTotal = inlineSegs
          .filter((s) => s.kind === 'add')
          .reduce((acc, s) => acc + Array.from(s.text).length, 0)
        const hasDeletes = inlineSegs.some((s) => s.kind === 'del')

        // Partial edit (< 65% changed): Human-like sequential select -> delete -> type in place!
        const editHunks = groupInlineDiffIntoHunks(inlineSegs)
        const totalEditHunks = editHunks.filter((h) => h.kind === 'edit').length
        if (changeRatio < 0.65 && totalEditHunks > 0) {
          animState.phase = 'writing'
          animState.showStream = true
          animState.streamText = ''

          const selectMs = remainingUnits >= 2 ? 90 : totalEditHunks >= 6 ? 140 : 220
          const betweenHunksMs = remainingUnits >= 2 ? 30 : totalEditHunks >= 6 ? 55 : 90
          const baseCharMs = remainingUnits >= 2 ? 20 : totalEditHunks >= 6 ? 28 : 40
          const charsPerTick = Math.max(
            1,
            Math.ceil((Math.max(1, addedTotal) * baseCharMs) / 2600)
          )

          const syncModifyHeight = async () => {
            await nextTick()
            if (streamPreviewEl.value) {
              // Keep overlay >= modifyOrigBoxH during editing so old paragraph lines underneath never peek out
              const neededH = Math.max(36, modifyOrigBoxH, streamPreviewEl.value.offsetHeight + 12)
              animState.boxHeight = neededH
              if (modifyAnchorEl) {
                setInsertSpacer(modifyAnchorEl, Math.max(0, neededH - modifyOrigBoxH))
              }
            }
          }

          let eIdx = 0
          for (const h of editHunks) {
            if (h.kind !== 'edit') continue
            if (animState.runId !== myRunId || animState.skip) break
            while (animState.paused && animState.runId === myRunId) await sleep(80)

            const del = h.delText || ''
            const add = h.addText || ''

            // Step 1: Select old text (like dragging selection before Backspace)
            if (del) {
              animState.streamHtml = renderHumanInlineEditHtml(editHunks, eIdx, 'select', 0)
              await syncModifyHeight()
              await sleep(selectMs)
              if (animState.runId !== myRunId || animState.skip) break
            }

            // Step 2: Delete selected text & type new text at the live caret
            const addChars = Array.from(add)
            if (addChars.length === 0) {
              // Pure deletion: show caret cleanly at deletion point
              animState.streamHtml = renderHumanInlineEditHtml(editHunks, eIdx, 'type', 0)
              await syncModifyHeight()
              await sleep(betweenHunksMs)
            } else {
              for (let c = charsPerTick; c <= addChars.length + charsPerTick - 1; c += charsPerTick) {
                if (animState.runId !== myRunId || animState.skip) break
                while (animState.paused && animState.runId === myRunId) await sleep(80)
                animState.streamHtml = renderHumanInlineEditHtml(
                  editHunks,
                  eIdx,
                  'type',
                  Math.min(addChars.length, c)
                )
                await syncModifyHeight()
                await sleep(baseCharMs)
              }
              await sleep(betweenHunksMs)
            }
            eIdx += 1
          }

          // Final resting frame before commit: text & line-breaks match Muya 100% (zero jump!)
          if (animState.runId === myRunId && !animState.skip) {
            animState.streamHtml = renderHumanInlineEditHtml(editHunks, totalEditHunks, 'done', 0)
            await syncModifyHeight()
            await sleep(120)
          }
        } else {
          // Substantial rewrite (>= 65% changed): smooth streaming rewrite with in-flow spacer
          const chars = Array.from(unitMd)
          const totalChars = chars.length
          animState.phase = 'writing'
          animState.streamHtml = ''
          animState.showStream = totalChars > 0
          const baseMsPerChar = remainingUnits >= 3 ? 16 : remainingUnits >= 1 ? 30 : 48
          const maxDurationMs = remainingUnits >= 3 ? 900 : remainingUnits >= 1 ? 1800 : 3200
          const charsPerTick = Math.max(1, Math.ceil((totalChars * baseMsPerChar) / maxDurationMs))
          const tickDelay = Math.max(
            22,
            Math.min(baseMsPerChar, Math.round(maxDurationMs / Math.max(1, totalChars / charsPerTick)))
          )
          for (let i = charsPerTick; i <= totalChars + charsPerTick - 1; i += charsPerTick) {
            if (animState.runId !== myRunId || animState.skip) break
            while (animState.paused && animState.runId === myRunId) await sleep(80)
            const count = Math.min(totalChars, i)
            animState.streamText = chars.slice(0, count).join('')
            await nextTick()
            if (streamPreviewEl.value) {
              const neededH = streamPreviewEl.value.offsetHeight + 10
              if (neededH > animState.boxHeight) {
                animState.boxHeight = neededH
                if (modifyAnchorEl) setInsertSpacer(modifyAnchorEl, neededH - modifyOrigBoxH)
              }
            }
            const lastChar = chars[count - 1] || ''
            const extraPause = charsPerTick === 1 && PUNCT_PAUSE_RE.test(lastChar) ? 70 : 0
            await sleep(tickDelay + extraPause)
          }
        }
      } else if (unitType === 'insert_block' && !animState.skip) {
        const chars = Array.from(unitMd)
        const totalChars = chars.length
        animState.phase = 'writing'
        animState.streamHtml = ''
        animState.showStream = totalChars > 0

        // Adaptive pacing for consecutive multi-paragraph inserts (smooth & brisk when 3+ queued)
        const baseMsPerChar = remainingUnits >= 3 ? 16 : remainingUnits >= 1 ? 28 : 46
        const maxDurationMs = remainingUnits >= 3 ? 850 : remainingUnits >= 1 ? 1600 : 2800
        const charsPerTick = Math.max(1, Math.ceil((totalChars * baseMsPerChar) / maxDurationMs))
        const tickDelay = Math.max(
          22,
          Math.min(baseMsPerChar, Math.round(maxDurationMs / Math.max(1, totalChars / charsPerTick)))
        )

        for (let i = charsPerTick; i <= totalChars + charsPerTick - 1; i += charsPerTick) {
          if (animState.runId !== myRunId || animState.skip) break
          while (animState.paused && animState.runId === myRunId) await sleep(80)
          const count = Math.min(totalChars, i)
          animState.streamText = chars.slice(0, count).join('')
          await nextTick()
          if (streamPreviewEl.value) {
            const neededH = streamPreviewEl.value.offsetHeight + 10
            if (neededH > animState.boxHeight) {
              animState.boxHeight = neededH
              if (activeInsertSpacerEl) setInsertSpacer(activeInsertSpacerEl, animState.boxHeight + 16)
            }
          }
          const lastChar = chars[count - 1] || ''
          const extraPause = charsPerTick === 1 && PUNCT_PAUSE_RE.test(lastChar) ? 70 : 0
          await sleep(tickDelay + extraPause)
        }
      } else if (unitType === 'delete_block' && !animState.skip) {
        animState.phase = 'writing'
        await sleep(280)
      }

      if (animState.runId !== myRunId) return

      animState.showStream = false
      animState.streamText = ''
      animState.streamHtml = ''

      // Commit THIS single diff unit immediately to Muya so it becomes the real DOM anchor for the next unit!
      if (unitType === 'delete_block') {
        blocks.splice(unitIdx, 1)
      } else if (unitType === 'insert_block') {
        blocks.splice(unitIdx, 0, unitMd)
      } else {
        blocks[unitIdx] = unitMd
      }

      const isLastUnit = u === diffUnits.length - 1
      if (isLastUnit && op.resultingMarkdown !== undefined) {
        store.markdown = op.resultingMarkdown
        setMuyaMarkdownQuietly(op.resultingMarkdown, true)
      } else {
        const progressiveMd = blocks.join('\n\n') + '\n'
        store.markdown = progressiveMd
        setMuyaMarkdownQuietly(progressiveMd, true)
        if (isLastUnit && op.resultingMarkdown === undefined) {
          await store.onEditorMarkdownChanged(progressiveMd, true)
        }
      }

      clearInsertSpacer()
      await nextTick()
      await waitForLayoutFrame()
      recordPendingChange(unitType, unitIdx, unitOldMd, unitMd, op.label)
      marginTick.value += 1

      if (!animState.skipAll) {
        animState.phase = 'settled'
        positionOverlayOnBlock(
          unitIdx,
          '已更新',
          '',
          false,
          unitType === 'delete_block' ? undefined : unitMd,
          blocks.length
        )
        await sleep(remainingUnits > 0 ? 220 : 520)
      }
    }
  }

  if (animState.runId === myRunId) {
    clearInsertSpacer()
    if (!animState.skipAll && animState.visible && store.pendingDocOps.length === 0) {
      animState.phase = 'fading'
      await sleep(480)
    }
    if (animState.runId === myRunId) {
      animState.running = false
      animState.visible = false
      animState.skipAll = false
      animState.activeBlockIndex = -1
      animState.showStream = false
      animState.streamText = ''
      animState.streamHtml = ''
      marginTick.value += 1
      if (store.pendingDocOps.length > 0) {
        void drainOpQueue()
      }
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
    if (typeof muyaInstance.format === 'function') {
      muyaInstance.format(type)
      return
    }
    const sel = muyaInstance.editor.selection.getTextSelection()
    const block = (sel?.anchor?.block || muyaInstance.editor.activeContentBlock) as unknown as {
      format?: (t: string) => void
    } | null
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
  emit('comment-sent')
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

async function flushPendingSave() {
  if (muyaInstance && !suppressMuyaChange) {
    try {
      muyaInstance.flush()
    } catch {
      // ignore
    }
  }
  if (saveDebounceTimer) {
    clearTimeout(saveDebounceTimer)
    saveDebounceTimer = null
  }
  const item = pendingSave
  if (!item) return
  pendingSave = null
  await store.onEditorMarkdownChanged(item.markdown, true, {
    rootPath: item.rootPath,
    docRelPath: item.docRelPath
  })
  if (
    store.activeWorkspace?.rootPath === item.rootPath &&
    store.activeDocRelPath === item.docRelPath
  ) {
    flashSavedHint()
  }
}

async function saveNow(showFeedback = false) {
  menuOpen.value = false
  if (!store.activeWorkspace || !store.activeDocRelPath) return
  const targetRoot = store.activeWorkspace.rootPath
  const targetRel = store.activeDocRelPath
  if (pendingSave && (pendingSave.rootPath !== targetRoot || pendingSave.docRelPath !== targetRel)) {
    await flushPendingSave()
  } else {
    if (saveDebounceTimer) {
      clearTimeout(saveDebounceTimer)
      saveDebounceTimer = null
    }
    pendingSave = null
  }
  const md = muyaInstance ? muyaInstance.getMarkdown() : store.markdown
  await store.onEditorMarkdownChanged(md, true, {
    rootPath: targetRoot,
    docRelPath: targetRel
  })
  flashSavedHint()
  if (showFeedback) {
    store.showToast(`已保存：${targetRel}`)
  }
}

function scrollActiveFindMatchIntoView() {
  nextTick(() => {
    const container = muyaContainerEl.value
    if (!container) return
    const active =
      (container.querySelector('.mu-highlight') as HTMLElement | null) ||
      (container.querySelector('.mu-selection') as HTMLElement | null)
    if (active) {
      active.scrollIntoView({ block: 'center', behavior: 'smooth' })
    }
  })
}

function runDocumentFind() {
  if (!muyaInstance) {
    findBar.index = -1
    findBar.total = 0
    return
  }
  const q = findBar.query
  if (!q) {
    muyaInstance.search('')
    findBar.index = -1
    findBar.total = 0
    return
  }
  const res = muyaInstance.search(q, {
    isCaseSensitive: false,
    isWholeWord: false,
    isRegexp: false,
    selectHighlight: true
  })
  findBar.total = res.matches.length
  findBar.index = res.index
  if (res.matches.length > 0) {
    scrollActiveFindMatchIntoView()
  }
}

function stepDocumentFind(action: 'previous' | 'next') {
  if (!muyaInstance || !findBar.query) return
  const res = muyaInstance.find(action)
  findBar.total = res.matches.length
  findBar.index = res.index
  if (res.matches.length > 0) {
    scrollActiveFindMatchIntoView()
  }
}

function openFindBar() {
  if (!store.activeDocRelPath || !isMarkdownFile.value) return
  const sel = window.getSelection()
  if (
    sel &&
    !sel.isCollapsed &&
    muyaContainerEl.value &&
    sel.anchorNode &&
    muyaContainerEl.value.contains(sel.anchorNode)
  ) {
    const selected = sel.toString().trim()
    if (selected && !selected.includes('\n') && selected.length <= 80) {
      findBar.query = selected
    }
  }
  findBar.open = true
  nextTick(() => {
    findInputEl.value?.focus()
    findInputEl.value?.select()
    if (findBar.query) {
      runDocumentFind()
    }
  })
}

function closeFindBar() {
  findBar.open = false
  findBar.index = -1
  findBar.total = 0
  if (muyaInstance) {
    try {
      muyaInstance.search('')
    } catch {}
  }
}

const onGlobalKeydown = (e: KeyboardEvent) => {
  const mod = e.ctrlKey || e.metaKey
  const key = e.key.toLowerCase()
  const hasOpenModal = Boolean(
    document.querySelector('.modal-backdrop, .settings-backdrop, .history-backdrop, [role="dialog"]')
  )
  if (mod && !e.altKey && !e.shiftKey && key === 's') {
    if (!hasOpenModal && store.activeDocRelPath) {
      e.preventDefault()
      void saveNow(true)
    }
    return
  }
  if (mod && !e.altKey && !e.shiftKey && key === 'w') {
    if (!hasOpenModal && store.activeDocRelPath) {
      e.preventDefault()
      void store.closeTab(store.activeDocRelPath)
      return
    }
  }
  if (mod && !e.altKey && !e.shiftKey && key === 'f') {
    if (!hasOpenModal && store.activeDocRelPath && isMarkdownFile.value) {
      e.preventDefault()
      openFindBar()
      return
    }
  }
  if (key === 'escape' && findBar.open) {
    e.preventDefault()
    closeFindBar()
    return
  }

  // Global Undo / Redo for the active Markdown document (skip when empty state, modal open, or native input focused)
  if (
    !e.defaultPrevented &&
    !hasOpenModal &&
    mod &&
    !e.altKey &&
    muyaInstance &&
    Boolean(store.activeDocRelPath) &&
    isMarkdownFile.value &&
    (key === 'z' || key === 'y')
  ) {
    const active = document.activeElement as HTMLElement | null
    const inNativeInput = Boolean(
      active &&
        (active.tagName === 'INPUT' ||
          active.tagName === 'TEXTAREA' ||
          active.tagName === 'SELECT' ||
          (active.isContentEditable &&
            (!muyaContainerEl.value || !muyaContainerEl.value.contains(active))))
    )
    if (!inNativeInput) {
      if (key === 'z' && !e.shiftKey) {
        e.preventDefault()
        muyaInstance.undo()
        return
      }
      if (key === 'y' || (key === 'z' && e.shiftKey)) {
        e.preventDefault()
        muyaInstance.redo()
        return
      }
    }
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

let scrollResizeObs: ResizeObserver | null = null

function handleDocRenamed(oldRelPath: string, newRelPath: string) {
  const root = store.activeWorkspace?.rootPath
  if (!root) return
  const prefix = `${root}::`
  const rewriteKey = (k: string): string => {
    if (!k.startsWith(prefix)) return k
    const rel = k.slice(prefix.length)
    if (rel === oldRelPath) return `${prefix}${newRelPath}`
    if (rel.startsWith(oldRelPath + '/')) {
      return `${prefix}${newRelPath}${rel.slice(oldRelPath.length)}`
    }
    return k
  }
  for (const [k, v] of Array.from(docHistoryCache.entries())) {
    const nextK = rewriteKey(k)
    if (nextK !== k) {
      docHistoryCache.delete(k)
      docHistoryCache.set(nextK, v)
    }
  }
  for (const [k, v] of Array.from(docScrollMap.entries())) {
    const nextK = rewriteKey(k)
    if (nextK !== k) {
      docScrollMap.delete(k)
      docScrollMap.set(nextK, v)
    }
  }
  if (lastLoadedDocKey) {
    lastLoadedDocKey = rewriteKey(lastLoadedDocKey)
  }
}

onMounted(() => {
  store.registerEditorFlush(flushPendingSave)
  store.registerEditorRename(handleDocRenamed)
  document.addEventListener('click', onDocClick)
  window.addEventListener('keydown', onGlobalKeydown)
  if (scrollEl.value) {
    scrollPaneWidth.value = scrollEl.value.clientWidth || 1000
    scrollResizeObs = new ResizeObserver(() => {
      if (!scrollEl.value) return
      // Use clientWidth (includes padding!) so toggling .with-comment-rail padding NEVER causes oscillation!
      const nextW = scrollEl.value.clientWidth
      if (Math.abs(nextW - scrollPaneWidth.value) >= 2) {
        scrollPaneWidth.value = nextW
        marginTick.value += 1
      }
    })
    scrollResizeObs.observe(scrollEl.value)
  }
  if (store.activeDocRelPath) {
    mountMuya(store.markdown)
    lastLoadedDocKey = getDocCacheKey(store.activeWorkspace?.rootPath, store.activeDocRelPath)
  }
})

onUnmounted(() => {
  void flushPendingSave()
  store.registerEditorFlush(null)
  store.registerEditorRename(null)
  document.removeEventListener('click', onDocClick)
  window.removeEventListener('keydown', onGlobalKeydown)
  if (scrollResizeObs) {
    scrollResizeObs.disconnect()
    scrollResizeObs = null
  }
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
  transition: padding-right 0.2s ease;
}
.muya-scroll.with-comment-rail {
  padding: 36px 316px 120px 44px;
}
.muya-scroll.with-comment-rail.compact-comment-rail {
  padding: 36px 266px 120px 28px;
}
.margin-gutter {
  position: absolute;
  top: 0;
  right: -56px;
  width: 44px;
  pointer-events: none;
}
.margin-gutter.rail-mode {
  right: -292px;
  width: 264px;
}
.muya-scroll.compact-comment-rail .margin-gutter.rail-mode {
  right: -248px;
  width: 228px;
}
.margin-thread-card {
  position: absolute;
  left: 0;
  right: 0;
  pointer-events: auto;
  background: var(--bg-canvas);
  border: 1px solid var(--border-hairline);
  border-radius: 13px;
  padding: 12px 14px;
  box-shadow: var(--shadow-card);
  cursor: pointer;
  transition:
    top 0.22s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.margin-thread-card:hover {
  border-color: var(--border-soft);
  box-shadow: var(--shadow-pop);
}
.margin-thread-card.active {
  border-color: var(--border-soft);
  box-shadow: var(--shadow-pop);
  transform: translateX(-4px);
  z-index: 25;
}
.mtc-messages {
  display: flex;
  flex-direction: column;
  gap: 11px;
  max-height: 360px;
  overflow-y: auto;
  padding-right: 2px;
}
.mtc-msg {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.mtc-msg-head {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
}
.mtc-avatar {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 600;
  color: #ffffff;
  flex-shrink: 0;
}
.mtc-avatar.user {
  background: #d14d41;
}
.mtc-avatar.agent {
  background: var(--accent);
}
.mtc-author {
  font-weight: 600;
  color: var(--ink-primary);
}
.mtc-time {
  font-size: 11px;
  color: var(--ink-faint);
}
.mtc-msg-body {
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--ink-body);
  word-break: break-word;
  position: relative;
}
.mtc-msg-body.collapsed {
  max-height: 116px;
  overflow: hidden;
  -webkit-mask-image: linear-gradient(180deg, #000 66%, transparent 100%);
  mask-image: linear-gradient(180deg, #000 66%, transparent 100%);
}
.mtc-expand-btn {
  align-self: flex-start;
  padding: 1px 6px;
  margin-top: 2px;
  border-radius: 5px;
  font-size: 11px;
  color: var(--accent);
  background: transparent;
}
.mtc-expand-btn:hover {
  background: var(--accent-soft);
}
.comment-md :deep(p) {
  margin: 0 0 6px;
}
.comment-md :deep(p:last-child) {
  margin-bottom: 0;
}
.comment-md :deep(ul),
.comment-md :deep(ol) {
  margin: 4px 0 6px;
  padding-left: 16px;
}
.comment-md :deep(li) {
  margin: 2px 0;
}
.comment-md :deep(strong) {
  font-weight: 600;
  color: var(--ink-primary);
}
.comment-md :deep(code) {
  font-family: Consolas, 'JetBrains Mono', monospace;
  font-size: 11.5px;
  padding: 1px 4px;
  border-radius: 4px;
  background: var(--bg-subtle);
}
.comment-md :deep(.mtc-mention) {
  color: #2563eb;
  font-weight: 500;
}
[data-theme='dark'] .comment-md :deep(.mtc-mention) {
  color: #60a5fa;
}
.mtc-reply-box {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border-hairline);
  display: flex;
  flex-direction: column;
  gap: 7px;
}
.mtc-reply-input {
  width: 100%;
  padding: 6px 9px;
  border-radius: 8px;
  border: 1px solid var(--border-soft);
  background: var(--bg-sidebar);
  color: var(--ink-primary);
  font-size: 12px;
  outline: none;
}
.mtc-reply-input:focus {
  border-color: var(--accent);
}
.mtc-reply-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.mtc-resolve-btn {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--ink-muted);
  background: var(--bg-subtle);
}
.mtc-resolve-btn:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.mtc-send-btn {
  padding: 3px 10px;
  border-radius: 6px;
  font-size: 11.5px;
  font-weight: 500;
  background: var(--ink-primary);
  color: var(--bg-canvas);
}
.mtc-send-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
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
.find-bar {
  position: absolute;
  top: 56px;
  right: 20px;
  z-index: 45;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px 6px 10px;
  border-radius: 10px;
  background: var(--float-bg-color);
  border: 1px solid var(--border-soft);
  box-shadow: var(--shadow-pop);
}
.find-icon {
  color: var(--ink-muted);
  flex-shrink: 0;
}
.find-input {
  width: 190px;
  border: none;
  outline: none;
  background: transparent;
  color: var(--ink-primary);
  font-size: 13px;
  line-height: 1.4;
}
.find-count {
  font-size: 11.5px;
  color: var(--ink-muted);
  white-space: nowrap;
  padding: 0 4px;
  font-variant-numeric: tabular-nums;
}
.find-nav-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-secondary, var(--ink-primary));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}
.find-nav-btn:hover:not(:disabled) {
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.find-nav-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
</style>
