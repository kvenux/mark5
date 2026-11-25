<template>
  <section
    class="chat-pane"
    :class="{ expanded: !docVisible }"
    :style="!docVisible ? undefined : { width: `${paneWidth}px` }"
  >
    <!-- Minimal Top Header -->
    <header class="chat-header">
      <div class="chat-header-left">
        <button v-if="!sidebarVisible" class="icon-btn" title="展开侧栏" @click="$emit('expand-sidebar')">
          <PanelLeftOpen :size="16" :stroke-width="1.75" />
        </button>
        <span class="chat-title" :title="store.activeSession?.title || '新会话'">
          {{ store.activeSession?.title || '新会话' }}
        </span>
      </div>

      <div class="chat-header-right">
        <button
          v-if="store.activeSession?.docRelPath && docVisible"
          class="header-doc-pill"
          :title="store.activeSession.docRelPath"
          @click="openDoc(store.activeSession.docRelPath)"
        >
          <FileText :size="12" :stroke-width="1.75" />
          <span>{{ shortName(store.activeSession.docRelPath) }}</span>
        </button>
        <button
          v-if="!docVisible && (store.activeDocRelPath || store.activeSession?.docRelPath)"
          class="btn-outline"
          @click="openPrimaryDoc"
        >
          打开文档
        </button>
      </div>
    </header>

    <!-- Message Stream -->
    <div
      ref="listEl"
      class="chat-stream"
      @scroll="onStreamScroll"
      @click="onStreamClick"
    >
      <div class="chat-inner">
        <!-- Minimal Editorial Empty State (P1-1) -->
        <div v-if="!messages.length" class="chat-empty">
          <div class="empty-heading">开始写作协作</div>
          <div v-if="contextDocRelPath" class="empty-doc-tag">
            <FileText :size="13" :stroke-width="1.75" />
            <span>当前关联：{{ contextDocRelPath }}</span>
          </div>
          <div class="empty-starters">
            <button
              v-for="item in starterPrompts"
              :key="item"
              class="starter-pill"
              @click="sendStarter(item)"
            >
              {{ item }}
            </button>
          </div>
        </div>

        <div
          v-for="(m, idx) in messages"
          :key="m.id"
          class="turn-row"
          :class="m.role"
        >
          <!-- User Turn (with Structured Quote Blocks & Hover Actions) -->
          <div v-if="m.role === 'user'" class="user-turn">
            <div class="user-bubble">
              <div v-if="m.quotes && m.quotes.length" class="user-quotes">
                <template v-for="(q, qi) in m.quotes" :key="qi">
                  <div
                    v-if="q.kind === 'pasted'"
                    class="pasted-card in-msg"
                    title="点击查看完整粘贴内容"
                    @click="openPastedPreview(q.text)"
                  >
                    <div class="pasted-card-preview">{{ q.text }}</div>
                    <div class="pasted-card-foot">
                      <span class="pasted-badge">PASTED</span>
                      <span class="pasted-meta">{{ formatCharCount(q.text) }}</span>
                    </div>
                  </div>
                  <div
                    v-else
                    class="user-quote-item"
                    :class="{ clickable: Boolean(q.docRelPath) }"
                    @click="q.docRelPath && openDoc(q.docRelPath)"
                  >
                    <span v-if="q.docRelPath" class="uq-doc">{{ shortName(q.docRelPath) }}</span>
                    <span class="uq-text">{{ q.text }}</span>
                  </div>
                </template>
              </div>
              <div v-if="m.text" class="user-text">{{ m.text }}</div>
            </div>
            <div class="msg-actions user-actions">
              <span class="msg-time">{{ formatClock(m.timestamp) }}</span>
              <button class="act-btn" title="复制" @click="copyText(m.text)">
                <Copy :size="12" :stroke-width="1.75" />
              </button>
              <button class="act-btn" title="重新编辑" @click="reEditUserMsg(m)">
                <Pencil :size="12" :stroke-width="1.75" />
              </button>
            </div>
          </div>

          <!-- Status Line -->
          <div v-else-if="m.role === 'status'" class="status-line">{{ m.text }}</div>

          <!-- Error Banner with Recovery Actions (P0-4) -->
          <div v-else-if="m.role === 'error'" class="error-banner">
            <div class="error-text">{{ m.text }}</div>
            <div class="error-actions">
              <button
                class="err-act-btn"
                :disabled="store.agentBusy"
                @click="retryLastTurn(idx)"
              >
                <RotateCcw :size="12" :stroke-width="1.8" />
                <span>重试</span>
              </button>
              <button class="err-act-btn" @click="copyText(m.text)">
                <Copy :size="12" :stroke-width="1.8" />
                <span>复制</span>
              </button>
            </div>
          </div>

          <!-- Agent Turn -->
          <div v-else class="agent-turn">
            <!-- Collapsible Live Tool Trace (P0-3, P0-4, P2-1) -->
            <div v-if="m.tools && m.tools.length" class="tool-trace-wrap">
              <button
                class="tool-summary-btn"
                :title="summarizeTools(m, false)"
                @click="toggleTrace(m.id)"
              >
                <span class="trace-dot" :class="traceDotState(m)" />
                <span class="trace-summary-text">{{ summarizeTools(m, isTraceExpanded(m)) }}</span>
                <span class="chev">{{ isTraceExpanded(m) ? '▾' : '▸' }}</span>
              </button>
              <div v-if="isTraceExpanded(m)" class="tool-trace-list">
                <div
                  v-for="t in m.tools"
                  :key="t.id"
                  class="tool-item"
                  :class="{ 'is-error': t.state === 'error' }"
                >
                  <span class="tool-state" :class="t.state">
                    {{ t.state === 'done' ? '✓' : t.state === 'error' ? '✕' : '●' }}
                  </span>
                  <span class="tool-label" :title="t.detail || t.name">{{ t.detail || t.name }}</span>
                  <span v-if="t.durationSec !== undefined" class="tool-dur">
                    {{ t.durationSec.toFixed(1) }}s
                  </span>
                </div>
                <div
                  v-if="m.streaming && !m.tools.some((t) => t.state === 'running') && !(m.text && m.text.trim())"
                  class="tool-item"
                >
                  <span class="tool-state running">●</span>
                  <span class="tool-label">{{ store.agentPhase || '正在思考下一步...' }}</span>
                </div>
              </div>
            </div>

            <!-- 1-Line Live Activity Reminder (when thinking before any tool call) -->
            <div
              v-if="m.streaming && store.agentPhase && (!m.tools || !m.tools.length)"
              class="live-phase"
            >
              <span class="phase-dot" />
              <span>{{ store.agentPhase }}</span>
            </div>

            <!-- Rendered Markdown Agent Prose (P0-1) -->
            <div v-if="m.text && m.text.trim()" class="agent-prose md-prose">
              <div v-html="renderMarkdown(m.text)" />
              <span v-if="m.streaming" class="stream-caret" />
            </div>

            <!-- Per-Turn Inline Document Cards (P1-4) -->
            <div v-if="m.touchedDocs && m.touchedDocs.length" class="turn-doc-cards">
              <div
                v-for="rel in m.touchedDocs"
                :key="rel"
                class="doc-card"
                @click="openDoc(rel)"
              >
                <div class="doc-card-icon">
                  <FileText :size="16" :stroke-width="1.75" color="#2563eb" />
                </div>
                <div class="doc-card-meta">
                  <div class="doc-card-title">{{ rel }}</div>
                  <div class="doc-card-sub">Markdown · {{ store.activeWorkspace?.name }}</div>
                </div>
              </div>
            </div>

            <!-- Agent Message Action Bar (P1-2) -->
            <div v-if="!m.streaming && m.text && m.text.trim()" class="msg-actions agent-actions">
              <button class="act-btn" title="复制全文" @click="copyText(m.text)">
                <Copy :size="12" :stroke-width="1.75" />
                <span>复制</span>
              </button>
              <button
                class="act-btn"
                title="重新生成"
                :disabled="store.agentBusy"
                @click="retryLastTurn(idx)"
              >
                <RotateCcw :size="12" :stroke-width="1.75" />
                <span>重新生成</span>
              </button>
              <button
                v-if="canAppendToDoc"
                class="act-btn"
                title="插入到当前文档末尾"
                @click="store.appendToCurrentDocument(m.text)"
              >
                <FilePlus2 :size="12" :stroke-width="1.75" />
                <span>插入文档</span>
              </button>
              <span class="msg-time">{{ formatClock(m.timestamp) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Floating Scroll-to-Bottom Button (P1-5) -->
    <button
      v-if="!isNearBottom && messages.length > 0"
      class="scroll-bottom-btn"
      title="回到底部"
      @click="scrollToBottom(true)"
    >
      <ArrowDown :size="13" :stroke-width="2" />
      <span>回到底部</span>
    </button>

    <!-- Floating Rounded Composer Card -->
    <div class="composer-outer">
      <div class="composer-card">
        <!-- Multi-Quote Context Area (P2-2) -->
        <div v-if="store.quotedSnippets.length" class="quoted-wrap">
          <div v-if="store.quotedSnippets.length >= 2" class="quoted-head">
            <span>已引用 {{ store.quotedSnippets.length }} 段</span>
            <button class="quoted-clear" @click="store.clearQuotes()">清空</button>
          </div>
          <div class="quoted-list">
            <div v-for="q in store.quotedSnippets" :key="q.id" class="quoted-chip">
              <Quote :size="13" :stroke-width="1.75" class="quoted-ico" />
              <div class="quoted-body">
                <span v-if="q.docRelPath" class="quoted-doc">{{ shortName(q.docRelPath) }}</span>
                <span class="quoted-text">{{ q.text }}</span>
              </div>
              <button
                class="quoted-close"
                title="移除引用"
                @click="store.removeQuote(q.id)"
              >
                <X :size="12" :stroke-width="1.85" />
              </button>
            </div>
          </div>
        </div>

        <!-- Pasted Long-Text Cards (Claude Style) -->
        <div v-if="pastedCards.length" class="pasted-tray">
          <div
            v-for="card in pastedCards"
            :key="card.id"
            class="pasted-card"
            :title="`共 ${card.text.length} 字 · 点击查看或编辑`"
            @click="openPastedEditor(card)"
          >
            <button
              class="pasted-remove"
              title="移除粘贴卡片"
              @click.stop="removePastedCard(card.id)"
            >
              <X :size="11" :stroke-width="2" />
            </button>
            <div class="pasted-card-preview">{{ card.text }}</div>
            <div class="pasted-card-foot">
              <span class="pasted-badge">PASTED</span>
              <button
                class="pasted-edit-btn"
                title="编辑粘贴内容"
                @click.stop="openPastedEditor(card)"
              >
                <Pencil :size="12" :stroke-width="1.8" />
              </button>
            </div>
          </div>
        </div>

        <!-- Auto-Growing Textarea (P0-2) -->
        <textarea
          ref="composerEl"
          v-model="draft"
          rows="2"
          :placeholder="
            pastedCards.length
              ? '针对粘贴内容输入指令或问题…（Enter 发送）'
              : store.quotedSnippets.length
                ? '针对以上引用提问或下达修改指令…'
                : '输入指令或问题…（Enter 发送，Shift+Enter 换行）'
          "
          @keydown="onKeydown"
          @paste="onComposerPaste"
        />

        <!-- Composer Footer with Engine Selector (editable before 1st msg, locked after) + Attached Doc Context Pill -->
        <div class="composer-bar">
          <div class="composer-meta">
            <select
              v-if="messages.length === 0"
              class="engine-select"
              :value="currentEngine"
              title="选择本会话使用的 AI 引擎（发送首条消息后永久绑定）"
              @change="onEngineChange"
            >
              <option value="agy">AGY</option>
              <option value="claude">Claude Code</option>
              <option value="codex">Codex</option>
            </select>
            <span v-else class="engine-label" title="本会话已绑定的 AI 引擎">
              {{ engineLabel(currentEngine) }}
            </span>
            <div ref="modelPickerEl" class="model-picker-wrap">
              <button
                class="model-chip"
                :class="{ open: modelMenuOpen }"
                :title="`当前模型：${activeModelDisplay}（点击切换）`"
                @click.stop="toggleModelMenu"
              >
                <span class="model-chip-text">{{ activeModelDisplay }}</span>
                <ChevronDown :size="11" :stroke-width="1.9" class="model-chip-caret" />
              </button>
              <div v-if="modelMenuOpen" class="model-popover" @click.stop>
                <div class="model-pop-head">
                  <input
                    ref="modelFilterInputEl"
                    v-model="modelQuery"
                    class="model-filter-input"
                    placeholder="筛选或输入模型 ID…"
                    @keydown.enter.prevent="onModelFilterEnter"
                    @keydown.escape.prevent="modelMenuOpen = false"
                  />
                  <button
                    class="model-refresh-btn"
                    :disabled="store.modelsLoading[currentEngine]"
                    title="从 CLI 刷新可用模型列表"
                    @click="store.fetchEngineModels(currentEngine, true)"
                  >
                    <RefreshCw
                      :size="12"
                      :stroke-width="1.9"
                      :class="{ spin: store.modelsLoading[currentEngine] }"
                    />
                  </button>
                </div>
                <div class="model-pop-list">
                  <button
                    v-if="!modelQuery.trim()"
                    class="model-opt"
                    :class="{ active: !activeModelId }"
                    @click="pickModel('')"
                  >
                    <div class="model-opt-main">
                      <span class="model-opt-label">默认模型</span>
                      <span class="model-opt-sub">跟随 {{ engineLabel(currentEngine) }} CLI 默认配置</span>
                    </div>
                    <Check v-if="!activeModelId" :size="13" :stroke-width="2.2" class="model-opt-check" />
                  </button>

                  <div
                    v-if="store.modelsLoading[currentEngine] && filteredModels.length === 0"
                    class="model-pop-empty"
                  >
                    正在从 {{ engineLabel(currentEngine) }} CLI 获取模型列表…
                  </div>

                  <button
                    v-for="m in filteredModels"
                    :key="m.id"
                    class="model-opt"
                    :class="{ active: activeModelId === m.id }"
                    @click="pickModel(m.id)"
                  >
                    <div class="model-opt-main">
                      <span class="model-opt-label">{{ m.label }}</span>
                      <span v-if="m.label !== m.id" class="model-opt-sub">{{ m.id }}</span>
                    </div>
                    <Check v-if="activeModelId === m.id" :size="13" :stroke-width="2.2" class="model-opt-check" />
                  </button>

                  <button
                    v-if="
                      modelQuery.trim() &&
                      !filteredModels.some((m) => m.id.toLowerCase() === modelQuery.trim().toLowerCase())
                    "
                    class="model-opt custom-opt"
                    @click="pickModel(modelQuery.trim())"
                  >
                    <div class="model-opt-main">
                      <span class="model-opt-label">使用自定义模型</span>
                      <span class="model-opt-sub">{{ modelQuery.trim() }}</span>
                    </div>
                  </button>

                  <div
                    v-if="!store.modelsLoading[currentEngine] && filteredModels.length === 0 && !modelQuery.trim()"
                    class="model-pop-empty"
                  >
                    未获取到模型列表，可直接输入模型 ID 或点右上角刷新
                  </div>
                </div>
              </div>
            </div>
            <button
              v-if="contextDocRelPath"
              class="context-doc-chip"
              :title="`当前上下文文档：${contextDocRelPath}`"
              @click="openDoc(contextDocRelPath)"
            >
              <FileText :size="11" :stroke-width="1.75" />
              <span>{{ shortName(contextDocRelPath) }}</span>
            </button>
          </div>
          <button
            v-if="!store.agentBusy"
            class="send-circle"
            :disabled="!draft.trim() && pastedCards.length === 0"
            title="发送 (Enter)"
            @click="send"
          >
            <ArrowUp :size="15" :stroke-width="2" />
          </button>
          <button v-else class="stop-circle" title="停止" @click="abort">
            <Square :size="11" :stroke-width="2" fill="currentColor" />
          </button>
        </div>
      </div>
    </div>

    <!-- Pasted Content Preview / Edit Modal -->
    <div
      v-if="pastedModal.open"
      class="pasted-modal-mask"
      @click.self="closePastedModal"
    >
      <div class="pasted-modal">
        <div class="pasted-modal-head">
          <div class="pasted-modal-title">
            <span class="pasted-badge">PASTED</span>
            <span class="pasted-modal-stats">{{ formatCharCount(pastedModal.text) }}</span>
          </div>
          <button class="pasted-modal-close" title="关闭" @click="closePastedModal">
            <X :size="15" :stroke-width="1.9" />
          </button>
        </div>
        <textarea
          v-if="pastedModal.editable"
          v-model="pastedModal.text"
          class="pasted-modal-editor"
          rows="14"
        />
        <pre v-else class="pasted-modal-viewer">{{ pastedModal.text }}</pre>
        <div class="pasted-modal-foot">
          <button class="btn-outline" @click="copyText(pastedModal.text)">复制全文</button>
          <div class="pasted-modal-actions">
            <button class="btn-outline" @click="closePastedModal">
              {{ pastedModal.editable ? '取消' : '关闭' }}
            </button>
            <button v-if="pastedModal.editable" class="btn-primary" @click="savePastedModal">
              保存修改
            </button>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import {
  ArrowDown,
  ArrowUp,
  Check,
  ChevronDown,
  Copy,
  FilePlus2,
  FileText,
  PanelLeftOpen,
  Pencil,
  Quote,
  RefreshCw,
  RotateCcw,
  Square,
  X
} from 'lucide-vue-next'
import type { AgentEngine, ChatMessage } from '@shared/types'
import { useAppStore } from '@/stores/appStore'

withDefaults(
  defineProps<{
    sidebarVisible: boolean
    docVisible: boolean
    paneWidth?: number
  }>(),
  {
    paneWidth: 390
  }
)

const emit = defineEmits<{
  (e: 'expand-sidebar'): void
  (e: 'open-doc'): void
  (e: 'open-model-settings'): void
}>()

const store = useAppStore()
const draft = ref('')
const pastedCards = ref<Array<{ id: string; text: string }>>([])
const pastedModal = reactive<{
  open: boolean
  editable: boolean
  cardId: string | null
  text: string
}>({
  open: false,
  editable: false,
  cardId: null,
  text: ''
})
const PASTE_CHAR_THRESHOLD = 300
const PASTE_LINE_THRESHOLD = 6
const listEl = ref<HTMLElement | null>(null)
const composerEl = ref<HTMLTextAreaElement | null>(null)
const expandedTraces = reactive<Record<string, boolean>>({})
const isNearBottom = ref(true)
let appliedQuoteSeq = 0

const messages = computed(() => store.activeSession?.messages || [])
const currentEngine = computed<AgentEngine>(
  () => store.activeSession?.engine || store.preferredEngine || 'agy'
)
const modelMenuOpen = ref(false)
const modelQuery = ref('')
const modelPickerEl = ref<HTMLElement | null>(null)
const modelFilterInputEl = ref<HTMLInputElement | null>(null)

const activeModelId = computed(() => {
  const m = store.activeSession?.model || store.settings.models[currentEngine.value]
  return m && m.trim() ? m.trim() : ''
})
const activeModelDisplay = computed(() => {
  const id = activeModelId.value
  if (!id) return '默认模型'
  const found = (store.engineModels[currentEngine.value] || []).find((x) => x.id === id)
  return found ? found.label : id
})
const filteredModels = computed(() => {
  const list = store.engineModels[currentEngine.value] || []
  const q = modelQuery.value.trim().toLowerCase()
  if (!q) return list
  return list.filter(
    (m) => m.label.toLowerCase().includes(q) || m.id.toLowerCase().includes(q)
  )
})

function toggleModelMenu() {
  modelMenuOpen.value = !modelMenuOpen.value
  if (modelMenuOpen.value) {
    modelQuery.value = ''
    if ((store.engineModels[currentEngine.value] || []).length === 0) {
      void store.fetchEngineModels(currentEngine.value)
    }
    nextTick(() => modelFilterInputEl.value?.focus())
  }
}

async function pickModel(modelId: string) {
  await store.setActiveModel(currentEngine.value, modelId)
  modelMenuOpen.value = false
  modelQuery.value = ''
}

function onModelFilterEnter() {
  const q = modelQuery.value.trim()
  if (filteredModels.value.length > 0) {
    void pickModel(filteredModels.value[0].id)
  } else if (q) {
    void pickModel(q)
  }
}

function onDocClickForModelMenu(ev: MouseEvent) {
  if (!modelMenuOpen.value) return
  if (modelPickerEl.value && !modelPickerEl.value.contains(ev.target as Node)) {
    modelMenuOpen.value = false
  }
}

function onWindowKeydownForModals(ev: KeyboardEvent) {
  if (ev.key === 'Escape' && pastedModal.open) {
    ev.preventDefault()
    closePastedModal()
  }
}

onMounted(() => {
  document.addEventListener('click', onDocClickForModelMenu)
  window.addEventListener('keydown', onWindowKeydownForModals)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClickForModelMenu)
  window.removeEventListener('keydown', onWindowKeydownForModals)
})
const contextDocRelPath = computed(
  () => store.activeDocRelPath || store.activeSession?.docRelPath || null
)
const canAppendToDoc = computed(() =>
  Boolean(store.activeDocRelPath && /\.(md|markdown)$/i.test(store.activeDocRelPath))
)

const starterPrompts = computed(() =>
  contextDocRelPath.value
    ? ['通读当前文档并提出 3 条核心改进建议', '润色当前文档语言表达，使其更紧凑专业', '为当前文档补充一节风险与兜底方案']
    : ['帮我起草一份技术方案文档大纲', '创建一份项目复盘报告骨架', '整理一份上线发布检查清单']
)

// Safe Markdown renderer with code block language header & copy button (P0-1)
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
})
const defaultValidateLink = md.validateLink
md.validateLink = (url: string) => /^file:\/\//i.test(url) || defaultValidateLink(url)

const defaultFence =
  md.renderer.rules.fence ||
  ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options))

md.renderer.rules.fence = (tokens, idx, options, env, self) => {
  const token = tokens[idx]
  const lang = (token.info || '').trim().split(/\s+/)[0] || 'text'
  const rawHtml = defaultFence(tokens, idx, options, env, self)
  const encoded = encodeURIComponent(token.content)
  return `<div class="md-code-block"><div class="md-code-head"><span class="md-code-lang">${md.utils.escapeHtml(lang)}</span><button type="button" class="md-code-copy" data-copy="${encoded}">复制</button></div>${rawHtml}</div>`
}

function renderMarkdown(source: string): string {
  const raw = md.render(source || '')
  return DOMPurify.sanitize(raw, {
    ADD_ATTR: ['data-copy', 'target', 'rel'],
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|file):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$))/i
  })
}

function shortName(relPath: string): string {
  const parts = relPath.split(/[\\/]/)
  return parts[parts.length - 1] || relPath
}

function formatClock(ts: number): string {
  const d = new Date(ts)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

function isTraceExpanded(m: ChatMessage): boolean {
  return Boolean(expandedTraces[m.id] || m.streaming)
}

function toggleTrace(msgId: string) {
  expandedTraces[msgId] = !expandedTraces[msgId]
}

function traceDotState(m: ChatMessage): string {
  const list = m.tools || []
  if (m.streaming) return 'running'
  if (list.some((t) => t.state === 'error')) return 'error'
  return 'done'
}

function summarizeTools(m: ChatMessage, expanded: boolean): string {
  const list = m.tools || []
  const runningIdx = list.findIndex((t) => t.state === 'running')
  const failed = list.filter((t) => t.state === 'error').length

  // Concise count-based summary when expanded to avoid repeating the first tool item (P2-1)
  if (expanded) {
    if (runningIdx >= 0) return `执行中 · 第 ${runningIdx + 1} / ${list.length} 步`
    if (m.streaming) return `思考中 · 已完成 ${list.length} 步`
    if (failed > 0) return `执行了 ${list.length} 步操作（${failed} 步失败）`
    return `共执行 ${list.length} 步操作`
  }

  if (runningIdx >= 0) {
    const running = list[runningIdx]
    return `正在${running.detail || running.name}`
  }
  if (m.streaming) return `思考中（已完成 ${list.length} 步）`
  if (failed > 0) return `执行了 ${list.length} 步操作（${failed} 步失败）`
  const last = list[list.length - 1]
  return last?.detail ? `${last.detail}（共 ${list.length} 步）` : `已完成 ${list.length} 步操作`
}

function engineLabel(eng: AgentEngine) {
  if (eng === 'claude') return 'Claude Code'
  if (eng === 'codex') return 'Codex'
  return 'AGY'
}

function onEngineChange(e: Event) {
  const val = (e.target as HTMLSelectElement).value as AgentEngine
  void store.setActiveSessionEngine(val)
}

async function openDoc(relPath: string) {
  await store.openDocument(relPath)
  emit('open-doc')
}

async function openPrimaryDoc() {
  const target = store.activeDocRelPath || store.activeSession?.docRelPath
  if (target) {
    await store.openDocument(target)
  }
  emit('open-doc')
}

function syncComposerHeight() {
  const el = composerEl.value
  if (!el) return
  el.style.height = 'auto'
  const next = Math.min(180, Math.max(44, el.scrollHeight))
  el.style.height = `${next}px`
}

watch(draft, () => {
  nextTick(syncComposerHeight)
})

function applyPendingQuote() {
  const req = store.chatDraftRequest
  if (req.seq > appliedQuoteSeq && req.text) {
    appliedQuoteSeq = req.seq
    nextTick(() => {
      composerEl.value?.focus()
    })
  }
}

watch(() => store.chatDraftRequest.seq, applyPendingQuote)

onMounted(() => {
  applyPendingQuote()
  nextTick(syncComposerHeight)
})

function onComposerPaste(e: ClipboardEvent) {
  const raw = e.clipboardData?.getData('text/plain') || ''
  if (!raw) return
  const lineCount = raw.split(/\r?\n/).length
  if (raw.length >= PASTE_CHAR_THRESHOLD || lineCount >= PASTE_LINE_THRESHOLD) {
    e.preventDefault()
    pastedCards.value.push({
      id: `p-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      text: raw.trim()
    })
  }
}

function removePastedCard(id: string) {
  pastedCards.value = pastedCards.value.filter((c) => c.id !== id)
}

function openPastedEditor(card: { id: string; text: string }) {
  pastedModal.open = true
  pastedModal.editable = true
  pastedModal.cardId = card.id
  pastedModal.text = card.text
}

function openPastedPreview(text: string) {
  pastedModal.open = true
  pastedModal.editable = false
  pastedModal.cardId = null
  pastedModal.text = text
}

function closePastedModal() {
  pastedModal.open = false
}

function savePastedModal() {
  if (pastedModal.cardId) {
    const clean = pastedModal.text.trim()
    if (!clean) {
      removePastedCard(pastedModal.cardId)
    } else {
      const target = pastedCards.value.find((c) => c.id === pastedModal.cardId)
      if (target) target.text = clean
    }
  }
  pastedModal.open = false
}

function formatCharCount(text: string): string {
  const lines = text.split(/\r?\n/).length
  return `${text.length} 字 · ${lines} 行`
}

async function send() {
  const text = draft.value.trim()
  const cards = pastedCards.value.map((c) => ({ text: c.text, kind: 'pasted' as const }))
  const quotes = [
    ...cards,
    ...store.quotedSnippets.map((q) => ({
      text: q.text,
      docRelPath: q.docRelPath,
      kind: 'quote' as const
    }))
  ]
  if (!text && quotes.length === 0) return
  draft.value = ''
  pastedCards.value = []
  store.clearQuotes()
  nextTick(syncComposerHeight)
  isNearBottom.value = true
  await store.sendChatPrompt(text, quotes)
}

async function sendStarter(prompt: string) {
  if (store.agentBusy) return
  isNearBottom.value = true
  await store.sendChatPrompt(prompt)
}

async function retryLastTurn(beforeIdx: number) {
  if (store.agentBusy) return
  const list = messages.value
  for (let i = beforeIdx - 1; i >= 0; i--) {
    if (list[i].role === 'user') {
      isNearBottom.value = true
      await store.sendChatPrompt(list[i].text, list[i].quotes)
      return
    }
  }
}

function reEditUserMsg(m: ChatMessage) {
  draft.value = m.text
  if (m.quotes?.length) {
    for (const q of m.quotes) {
      if (q.kind === 'pasted') {
        pastedCards.value.push({
          id: `p-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          text: q.text
        })
      } else {
        store.quoteToChat(q.text)
      }
    }
  }
  nextTick(() => {
    syncComposerHeight()
    composerEl.value?.focus()
  })
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    store.showToast('已复制')
  } catch {
    store.showToast('复制失败')
  }
}

async function onStreamClick(e: MouseEvent) {
  const target = e.target as HTMLElement | null
  if (!target) return

  // Code block copy button delegation
  const copyBtn = target.closest('.md-code-copy') as HTMLElement | null
  if (copyBtn) {
    e.preventDefault()
    const raw = copyBtn.getAttribute('data-copy') || ''
    await copyText(decodeURIComponent(raw))
    return
  }

  // Intercept Markdown links pointing to local / workspace files
  const anchor = target.closest('a') as HTMLAnchorElement | null
  if (anchor) {
    const href = anchor.getAttribute('href') || ''
    if (!href) return
    if (/^https?:\/\//i.test(href)) {
      e.preventDefault()
      window.open(href, '_blank', 'noopener')
      return
    }
    e.preventDefault()
    const wsRoot = store.activeWorkspace?.rootPath.replace(/\\/g, '/') || ''
    let cleaned = decodeURIComponent(href).replace(/^file:\/\/\//i, '').replace(/\\/g, '/')
    if (wsRoot && cleaned.toLowerCase().startsWith(wsRoot.toLowerCase() + '/')) {
      cleaned = cleaned.slice(wsRoot.length + 1)
    }
    cleaned = cleaned.replace(/^\.\//, '')
    if (cleaned) {
      await openDoc(cleaned)
    }
  }
}

async function abort() {
  if (store.activeSessionId) {
    await window.mark5.abortAgentTurn(store.activeSessionId)
    store.agentBusy = false
    store.agentPhase = ''
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
    e.preventDefault()
    send()
  }
}

function onStreamScroll() {
  const el = listEl.value
  if (!el) return
  isNearBottom.value = el.scrollHeight - el.scrollTop - el.clientHeight <= 48
}

function scrollToBottom(smooth = false) {
  const el = listEl.value
  if (!el) return
  el.scrollTo({ top: el.scrollHeight, behavior: smooth ? 'smooth' : 'auto' })
  isNearBottom.value = true
}

watch(
  () => [
    store.activeSessionId,
    messages.value.length,
    messages.value.at(-1)?.text.length,
    messages.value.at(-1)?.tools?.length
  ],
  async (newVals, oldVals) => {
    const sessionChanged = oldVals && newVals[0] !== oldVals[0]
    await nextTick()
    if (sessionChanged || isNearBottom.value) {
      scrollToBottom(false)
    }
  }
)
</script>

<style scoped>
.chat-pane {
  width: 390px;
  min-width: 300px;
  max-width: 560px;
  background: var(--bg-canvas);
  border-right: 1px solid var(--border-hairline);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  position: relative;
}
.chat-pane.expanded {
  flex: 1;
  width: auto !important;
  max-width: none;
  border-right: none;
}
.chat-header {
  height: 46px;
  padding: 0 14px;
  border-bottom: 1px solid var(--border-hairline);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-shrink: 0;
}
.chat-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.chat-header-right {
  display: flex;
  align-items: center;
  gap: 6px;
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
.chat-title {
  font-size: 13.5px;
  font-weight: 500;
  color: var(--ink-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.header-doc-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 150px;
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 11.5px;
  color: var(--ink-muted);
  background: var(--bg-subtle);
}
.header-doc-pill span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.header-doc-pill:hover {
  color: var(--ink-primary);
  background: var(--bg-active);
}
.chat-stream {
  flex: 1;
  overflow-y: auto;
  padding: 20px 18px 28px;
}
.chat-inner {
  max-width: 640px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.chat-empty {
  margin: 56px auto 0;
  max-width: 320px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
}
.empty-heading {
  font-size: 15px;
  font-weight: 600;
  color: var(--ink-primary);
}
.empty-doc-tag {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--bg-subtle);
  font-size: 12px;
  color: var(--ink-muted);
  max-width: 100%;
}
.empty-doc-tag span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.empty-starters {
  display: flex;
  flex-direction: column;
  gap: 7px;
  width: 100%;
  margin-top: 4px;
}
.starter-pill {
  width: 100%;
  text-align: left;
  padding: 9px 12px;
  border-radius: 10px;
  border: 1px solid var(--border-hairline);
  background: var(--bg-sidebar);
  font-size: 12.5px;
  color: var(--ink-body);
  transition: border-color 0.15s ease, background 0.15s ease;
}
.starter-pill:hover {
  border-color: var(--border-soft);
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.turn-row.user {
  display: flex;
  justify-content: flex-end;
}
.user-turn {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  max-width: 88%;
}
.user-bubble {
  background: var(--bg-subtle);
  color: var(--ink-primary);
  padding: 10px 14px;
  border-radius: 14px;
  font-size: 13.5px;
  line-height: 1.6;
  word-break: break-word;
}
.user-quotes {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 7px;
}
.user-quote-item {
  border-left: 2px solid var(--accent);
  padding: 2px 0 2px 8px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--ink-muted);
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.user-quote-item.clickable {
  cursor: pointer;
}
.user-quote-item.clickable:hover .uq-text {
  color: var(--ink-primary);
}
.uq-doc {
  font-size: 10.5px;
  color: var(--ink-faint);
}
.uq-text {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.user-text {
  white-space: pre-wrap;
}
.agent-turn {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.tool-trace-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.tool-summary-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  align-self: flex-start;
  max-width: 100%;
  text-align: left;
  padding: 3px 6px 3px 2px;
  font-size: 12.5px;
  color: var(--ink-muted);
  border-radius: 6px;
}
.tool-summary-btn:hover {
  color: var(--ink-primary);
  background: var(--bg-hover);
}
.trace-summary-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.trace-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--success);
}
.trace-dot.running {
  background: var(--accent);
  animation: phasePulse 1s infinite;
}
.trace-dot.error {
  background: #dc2626;
}
.chev {
  font-size: 11px;
  color: var(--ink-faint);
  flex-shrink: 0;
}
.tool-trace-list {
  border-left: 1.5px solid var(--border-soft);
  margin-left: 5px;
  padding-left: 11px;
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.tool-item {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  font-size: 12px;
  color: var(--ink-muted);
  line-height: 1.45;
}
.tool-state {
  width: 12px;
  text-align: center;
  font-size: 11px;
  flex-shrink: 0;
  margin-top: 1px;
}
.tool-state.done {
  color: var(--success);
}
.tool-state.running {
  color: var(--accent);
  animation: phasePulse 1s ease-in-out infinite;
}
.tool-state.error {
  color: #dc2626;
}
.tool-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tool-item.is-error .tool-label {
  color: #991b1b;
  white-space: normal;
  word-break: break-word;
}
.tool-dur {
  font-size: 11px;
  color: var(--ink-faint);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}
.live-phase {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: var(--ink-muted);
}
.phase-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
  animation: phasePulse 1s infinite;
}
@keyframes phasePulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

/* Rendered Markdown Prose in Chat (P0-1) */
.agent-prose {
  font-size: 14px;
  line-height: 1.72;
  color: var(--ink-primary);
  word-break: break-word;
}
.md-prose :deep(p) {
  margin: 0 0 10px;
}
.md-prose :deep(p:last-child) {
  margin-bottom: 0;
}
.md-prose :deep(h1),
.md-prose :deep(h2),
.md-prose :deep(h3),
.md-prose :deep(h4) {
  margin: 14px 0 6px;
  font-weight: 600;
  line-height: 1.4;
  color: var(--ink-primary);
}
.md-prose :deep(h1) {
  font-size: 16.5px;
}
.md-prose :deep(h2) {
  font-size: 15px;
}
.md-prose :deep(h3) {
  font-size: 14px;
}
.md-prose :deep(ul),
.md-prose :deep(ol) {
  margin: 6px 0 10px;
  padding-left: 20px;
}
.md-prose :deep(li) {
  margin-bottom: 4px;
}
.md-prose :deep(blockquote) {
  margin: 8px 0;
  padding: 2px 0 2px 12px;
  border-left: 2.5px solid var(--border-soft);
  color: var(--ink-muted);
}
.md-prose :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12.5px;
  background: var(--bg-subtle);
  padding: 1px 5px;
  border-radius: 5px;
  color: var(--ink-primary);
}
.md-prose :deep(.md-code-block) {
  margin: 10px 0;
  border: 1px solid var(--border-hairline);
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-sidebar);
}
.md-prose :deep(.md-code-head) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 5px 10px;
  border-bottom: 1px solid var(--border-hairline);
  background: var(--bg-subtle);
  font-size: 11px;
  color: var(--ink-muted);
}
.md-prose :deep(.md-code-copy) {
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 5px;
  color: var(--ink-muted);
}
.md-prose :deep(.md-code-copy:hover) {
  background: var(--bg-active);
  color: var(--ink-primary);
}
.md-prose :deep(.md-code-block pre) {
  margin: 0;
  padding: 10px 12px;
  overflow-x: auto;
}
.md-prose :deep(.md-code-block pre code) {
  background: transparent;
  padding: 0;
  font-size: 12.5px;
  line-height: 1.55;
}
.md-prose :deep(a) {
  color: #2563eb;
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
}
.md-prose :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 8px 0;
  font-size: 12.5px;
}
.md-prose :deep(th),
.md-prose :deep(td) {
  border: 1px solid var(--border-hairline);
  padding: 5px 8px;
  text-align: left;
}
.md-prose :deep(th) {
  background: var(--bg-subtle);
  font-weight: 600;
}
.stream-caret {
  display: inline-block;
  width: 6px;
  height: 14px;
  margin-left: 3px;
  vertical-align: -2px;
  background: var(--accent);
  border-radius: 1px;
  animation: phasePulse 0.9s infinite;
}

/* Message Action Bars (P1-2) */
.msg-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
}
.user-actions {
  opacity: 0;
  transition: opacity 0.14s ease;
  margin-top: 4px;
}
.user-turn:hover .user-actions {
  opacity: 1;
}
.agent-actions {
  opacity: 0.72;
  transition: opacity 0.14s ease;
}
.agent-turn:hover .agent-actions {
  opacity: 1;
}
.act-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--ink-muted);
}
.act-btn:hover:not(:disabled) {
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.act-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.msg-time {
  font-size: 11px;
  color: var(--ink-faint);
  margin-left: 2px;
}

.status-line {
  font-size: 12px;
  color: var(--ink-muted);
  padding: 2px 0;
}
.error-banner {
  font-size: 12.5px;
  color: var(--error-ink);
  background: var(--error-bg);
  border: 1px solid var(--error-border);
  border-radius: 10px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.error-text {
  white-space: pre-wrap;
  word-break: break-word;
  line-height: 1.5;
}
.error-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.err-act-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 9px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--error-ink);
  background: var(--error-btn-bg);
  border: 1px solid var(--error-border);
}
.err-act-btn:hover:not(:disabled) {
  background: var(--error-btn-hover);
}
.turn-doc-cards {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 2px;
}
.doc-card {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 10px 12px;
  background: var(--bg-sidebar);
  border: 1px solid var(--border-hairline);
  border-radius: 12px;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.doc-card:hover {
  border-color: var(--border-soft);
  background: var(--bg-subtle);
}
.doc-card-icon {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: var(--doc-icon-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.doc-card-meta {
  min-width: 0;
}
.doc-card-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--ink-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.doc-card-sub {
  font-size: 11.5px;
  color: var(--ink-muted);
  margin-top: 2px;
}

/* Scroll-to-bottom floating pill (P1-5) */
.scroll-bottom-btn {
  position: absolute;
  left: 50%;
  bottom: 122px;
  transform: translateX(-50%);
  z-index: 20;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 12px;
  border-radius: 999px;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  box-shadow: var(--shadow-pop);
  font-size: 12px;
  color: var(--ink-primary);
}
.scroll-bottom-btn:hover {
  background: var(--bg-subtle);
}

.composer-outer {
  padding: 10px 18px 16px;
  background: var(--bg-canvas);
}
.composer-card {
  max-width: 640px;
  margin: 0 auto;
  border: 1px solid var(--border-soft);
  border-radius: 16px;
  padding: 10px 12px 8px 14px;
  background: var(--bg-canvas);
  box-shadow: var(--shadow-card);
  transition: border-color 0.15s ease;
}
.composer-card:focus-within {
  border-color: #c7c4ba;
}
.quoted-wrap {
  margin-bottom: 8px;
}
.quoted-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  color: var(--ink-muted);
  margin-bottom: 5px;
  padding: 0 2px;
}
.quoted-clear {
  font-size: 11px;
  color: var(--ink-muted);
  padding: 1px 6px;
  border-radius: 5px;
}
.quoted-clear:hover {
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.quoted-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  max-height: 132px;
  overflow-y: auto;
}
.quoted-chip {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  padding: 6px 8px;
  background: var(--bg-subtle);
  border-left: 2.5px solid var(--accent);
  border-radius: 6px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--ink-body);
}
.quoted-ico {
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--accent);
}
.quoted-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.quoted-doc {
  font-size: 10.5px;
  color: var(--ink-faint);
}
.quoted-text {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-word;
}
.quoted-close {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  padding: 0;
  border-radius: 5px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
}
.quoted-close:hover {
  background: var(--bg-active);
  color: var(--ink-primary);
}
.composer-card textarea {
  width: 100%;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  font-size: 14px;
  line-height: 1.55;
  color: var(--ink-primary);
  min-height: 44px;
  max-height: 180px;
  overflow-y: auto;
}
.composer-card textarea::placeholder {
  color: var(--ink-faint);
}
.composer-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 6px;
}
.composer-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
}
.engine-select {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--ink-body);
  padding: 2px 4px 2px 6px;
  border-radius: 6px;
  background: var(--bg-subtle);
  border: 1px solid var(--border-hairline);
  outline: none;
  cursor: pointer;
  flex-shrink: 0;
  max-width: 74px;
}
.engine-select:hover {
  border-color: var(--border-soft);
  color: var(--ink-primary);
}
.engine-label {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--ink-muted);
  padding: 2px 7px;
  border-radius: 6px;
  background: var(--bg-subtle);
  flex-shrink: 0;
}
.model-picker-wrap {
  position: relative;
  flex: 0 0 auto;
  min-width: 0;
  max-width: 148px;
}
.model-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--ink-muted);
  padding: 2px 7px;
  border-radius: 6px;
  border: 1px solid var(--border-hairline);
  background: transparent;
  max-width: 100%;
  min-width: 0;
}
.model-chip-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.model-chip-caret {
  flex-shrink: 0;
  opacity: 0.7;
  transition: transform 0.14s ease;
}
.model-chip.open .model-chip-caret {
  transform: rotate(180deg);
}
.model-chip:hover,
.model-chip.open {
  background: var(--bg-subtle);
  border-color: var(--border-soft);
  color: var(--ink-primary);
}
.model-popover {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  width: 280px;
  max-height: 320px;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  box-shadow: var(--shadow-pop);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  z-index: 80;
}
.model-pop-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px;
  border-bottom: 1px solid var(--border-hairline);
}
.model-filter-input {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  padding: 4px 8px;
  border-radius: 7px;
  border: 1px solid var(--border-hairline);
  background: var(--bg-sidebar);
  color: var(--ink-primary);
  outline: none;
}
.model-filter-input:focus {
  border-color: var(--accent);
}
.model-refresh-btn {
  width: 26px;
  height: 26px;
  padding: 0;
  border-radius: 7px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
  flex-shrink: 0;
}
.model-refresh-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.model-refresh-btn .spin {
  animation: mcSpin 0.8s linear infinite;
}
@keyframes mcSpin {
  to {
    transform: rotate(360deg);
  }
}
.model-pop-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.model-opt {
  width: 100%;
  text-align: left;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 7px 9px;
  border-radius: 8px;
  color: var(--ink-body);
}
.model-opt:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.model-opt.active {
  background: var(--accent-soft);
  color: var(--ink-primary);
}
.model-opt-main {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.model-opt-label {
  font-size: 12.5px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.model-opt-sub {
  font-size: 10.5px;
  color: var(--ink-muted);
  font-family: Consolas, 'JetBrains Mono', monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.model-opt-check {
  color: var(--accent);
  flex-shrink: 0;
}
.model-pop-empty {
  padding: 16px 10px;
  text-align: center;
  font-size: 11.5px;
  color: var(--ink-muted);
}
.context-doc-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: 1 1 0;
  min-width: 0;
  max-width: 160px;
  overflow: hidden;
  padding: 2px 6px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--ink-muted);
  background: transparent;
}
.context-doc-chip svg {
  flex-shrink: 0;
}
.context-doc-chip span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.context-doc-chip:hover {
  background: var(--bg-subtle);
  color: var(--ink-primary);
}
.send-circle,
.stop-circle {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.send-circle {
  background: var(--accent);
  color: #ffffff;
}
.send-circle:hover:not(:disabled) {
  background: var(--accent-hover);
}
.send-circle:disabled {
  background: var(--bg-active);
  color: var(--ink-faint);
  cursor: not-allowed;
}

/* Claude-style Pasted Content Card */
.pasted-tray {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 2px 2px 8px;
}
.pasted-card {
  position: relative;
  width: 142px;
  height: 114px;
  border-radius: 11px;
  border: 1px solid var(--border-soft);
  background: var(--bg-canvas);
  box-shadow: var(--shadow-card);
  padding: 10px 10px 8px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 6px;
  cursor: pointer;
  transition: border-color 0.14s ease, box-shadow 0.14s ease;
}
.pasted-card:hover {
  border-color: var(--ink-faint);
}
.pasted-card.in-msg {
  width: 178px;
  height: 98px;
  margin-bottom: 4px;
}
.pasted-card.in-msg .pasted-card-preview {
  -webkit-line-clamp: 3;
}
.pasted-card-preview {
  font-size: 11px;
  line-height: 1.42;
  color: var(--ink-body);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  word-break: break-word;
  white-space: pre-wrap;
}
.pasted-card-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.pasted-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--bg-subtle);
  color: var(--ink-muted);
  font-size: 9.5px;
  font-weight: 600;
  letter-spacing: 0.04em;
  line-height: 1.3;
}
.pasted-meta {
  font-size: 10px;
  color: var(--ink-faint);
}
.pasted-edit-btn {
  width: 22px;
  height: 22px;
  padding: 0;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
  background: var(--bg-subtle);
}
.pasted-edit-btn:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.pasted-remove {
  position: absolute;
  top: -6px;
  right: -6px;
  width: 18px;
  height: 18px;
  padding: 0;
  border-radius: 50%;
  border: 1px solid var(--border-soft);
  background: var(--bg-canvas);
  color: var(--ink-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.12s ease, background 0.12s ease;
  box-shadow: var(--shadow-card);
}
.pasted-card:hover .pasted-remove {
  opacity: 1;
}
.pasted-remove:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.pasted-modal-mask {
  position: fixed;
  inset: 0;
  z-index: 200;
  background: rgba(0, 0, 0, 0.36);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.pasted-modal {
  width: min(620px, 92vw);
  max-height: 82vh;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  box-shadow: var(--shadow-pop);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.pasted-modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-hairline);
}
.pasted-modal-title {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pasted-modal-stats {
  font-size: 12px;
  color: var(--ink-muted);
}
.pasted-modal-close {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 7px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
}
.pasted-modal-editor,
.pasted-modal-viewer {
  margin: 0;
  padding: 14px 16px;
  flex: 1;
  min-height: 260px;
  max-height: 55vh;
  overflow: auto;
  border: none;
  outline: none;
  resize: none;
  background: var(--bg-sidebar);
  color: var(--ink-body);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
}
.pasted-modal-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-top: 1px solid var(--border-hairline);
}
.pasted-modal-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.stop-circle {
  background: var(--accent);
  color: #ffffff;
}
</style>
