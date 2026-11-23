<template>
  <div class="settings-mask" @click.self="$emit('close')">
    <div class="settings-dialog" @click.stop>
      <aside class="settings-nav">
        <div class="settings-brand">设置</div>
        <button
          class="snav-item"
          :class="{ active: tab === 'appearance' }"
          @click="tab = 'appearance'"
        >
          <Palette :size="15" :stroke-width="1.75" />
          <span>外观与主题</span>
        </button>
        <button
          class="snav-item"
          :class="{ active: tab === 'backend' }"
          @click="tab = 'backend'"
        >
          <Cpu :size="15" :stroke-width="1.75" />
          <span>AI 后端与模型</span>
        </button>
        <button
          class="snav-item"
          :class="{ active: tab === 'writing' }"
          @click="tab = 'writing'"
        >
          <PenLine :size="15" :stroke-width="1.75" />
          <span>个人写作偏好</span>
        </button>
      </aside>

      <section class="settings-content">
        <header class="settings-head">
          <span>{{ tabTitle }}</span>
          <button class="icon-close" title="关闭 (Esc)" @click="$emit('close')">
            <X :size="16" :stroke-width="1.75" />
          </button>
        </header>

        <div class="settings-body">
          <!-- Appearance Tab -->
          <template v-if="tab === 'appearance'">
            <div class="s-group">
              <div class="s-label">界面主题</div>
              <div class="theme-grid">
                <button
                  v-for="t in themes"
                  :key="t.id"
                  class="theme-card"
                  :class="{ active: store.settings.theme === t.id }"
                  @click="store.updateSettings({ theme: t.id })"
                >
                  <div class="theme-swatch" :style="{ background: t.bg, borderColor: t.border }">
                    <span class="swatch-side" :style="{ background: t.side }" />
                    <span class="swatch-dot" :style="{ background: t.accent }" />
                  </div>
                  <span class="theme-name">{{ t.label }}</span>
                </button>
              </div>
            </div>

            <div class="s-group">
              <div class="s-label">编辑器正文字号</div>
              <div class="seg-row">
                <button
                  v-for="sz in [14, 15, 16, 17]"
                  :key="sz"
                  class="seg-btn"
                  :class="{ active: store.settings.editorFontSize === sz }"
                  @click="store.updateSettings({ editorFontSize: sz })"
                >
                  {{ sz }}px
                </button>
              </div>
            </div>

            <div class="s-group">
              <div class="s-label">文档纸面最大宽度</div>
              <div class="seg-row">
                <button
                  v-for="w in [680, 720, 840, 960]"
                  :key="w"
                  class="seg-btn"
                  :class="{ active: store.settings.editorMaxWidth === w }"
                  @click="store.updateSettings({ editorMaxWidth: w })"
                >
                  {{ w }}px
                </button>
              </div>
            </div>
          </template>

          <!-- Backend & Models Tab -->
          <template v-else-if="tab === 'backend'">
            <div class="s-group">
              <div class="s-row-between">
                <div class="s-label" style="margin: 0">默认无头引擎</div>
                <button class="btn-mini" :disabled="checkingCli" @click="runCheckCli">
                  <RefreshCw :size="12" :stroke-width="1.8" :class="{ spin: checkingCli }" />
                  <span>{{ checkingCli ? '检测中...' : '检测本地 CLI' }}</span>
                </button>
              </div>
              <div class="seg-row" style="margin-top: 8px">
                <button
                  v-for="eng in engines"
                  :key="eng.id"
                  class="seg-btn"
                  :class="{ active: store.settings.defaultEngine === eng.id }"
                  @click="store.updateSettings({ defaultEngine: eng.id })"
                >
                  {{ eng.label }}
                </button>
              </div>

              <div v-if="cliStatuses.length" class="cli-status-list">
                <div v-for="st in cliStatuses" :key="st.engine" class="cli-status-row">
                  <span class="cli-dot" :class="{ ok: st.available }" />
                  <span class="cli-name">{{ engineTitle(st.engine) }}</span>
                  <span class="cli-ver">{{ st.version }}</span>
                </div>
              </div>
            </div>

            <div v-for="eng in engines" :key="eng.id" class="s-group engine-card">
              <div class="s-label">{{ eng.label }} 模型参数（--model）</div>
              <input
                class="s-input"
                :value="store.settings.models[eng.id]"
                :placeholder="`留空则使用 ${eng.label} 默认模型，例如 ${eng.presets[1]}`"
                @input="onModelInput(eng.id, $event)"
              />
              <div class="preset-row">
                <button
                  class="preset-chip"
                  :class="{ active: !store.settings.models[eng.id] }"
                  @click="setModel(eng.id, '')"
                >
                  默认
                </button>
                <button
                  v-for="m in livePresetsFor(eng.id)"
                  :key="m.id"
                  class="preset-chip"
                  :class="{ active: store.settings.models[eng.id] === m.id }"
                  :title="m.id"
                  @click="setModel(eng.id, m.id)"
                >
                  {{ m.label }}
                </button>
              </div>
              <div class="s-sub-label">自定义可执行路径（可选，留空自动探测）</div>
              <input
                class="s-input"
                :value="store.settings.cliPaths[eng.id]"
                :placeholder="eng.pathHint"
                @input="onPathInput(eng.id, $event)"
              />
            </div>
          </template>

          <!-- Writing & Prompts Tab -->
          <template v-else>
            <div class="s-group">
              <div class="s-row-between">
                <div>
                  <div class="s-label">个人写作偏好</div>
                  <div class="s-sub-label" style="margin-top: 2px">
                    对 AGY、Claude Code、Codex 三个写作 Agent 统一生效，可随时修改或停用
                  </div>
                </div>
                <div class="writing-actions">
                  <button
                    class="btn-mini"
                    :class="{ 'btn-mini-active': store.settings.enablePersonalWritingPrompt }"
                    @click="togglePersonalPrompt"
                  >
                    {{ store.settings.enablePersonalWritingPrompt ? '已启用' : '已停用' }}
                  </button>
                  <button class="btn-mini" @click="resetPersonalPrompt">
                    <RotateCcw :size="12" :stroke-width="1.8" />
                    <span>恢复默认</span>
                  </button>
                </div>
              </div>
              <textarea
                class="s-textarea"
                :class="{ dimmed: !store.settings.enablePersonalWritingPrompt }"
                :value="store.settings.personalWritingPrompt"
                rows="14"
                placeholder="输入你的个人写作风格、结构模板、用词偏好..."
                @input="onPersonalPromptInput"
              />
            </div>
          </template>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Cpu, Palette, PenLine, RefreshCw, RotateCcw, X } from 'lucide-vue-next'
import type { AgentEngine, AppTheme, CliStatusItem } from '@shared/types'
import { DEFAULT_PERSONAL_WRITING_PROMPT } from '@shared/writingPrompts'
import { useAppStore } from '@/stores/appStore'

const props = withDefaults(
  defineProps<{
    initialTab?: 'appearance' | 'backend' | 'writing'
  }>(),
  {
    initialTab: 'appearance'
  }
)

const emit = defineEmits<{
  (e: 'close'): void
}>()

const store = useAppStore()
const tab = ref<'appearance' | 'backend' | 'writing'>(props.initialTab)
const tabTitle = computed(() => {
  if (tab.value === 'backend') return 'AI 后端与模型'
  if (tab.value === 'writing') return '个人写作偏好'
  return '外观与主题'
})
const checkingCli = ref(false)
const cliStatuses = ref<CliStatusItem[]>([])

const themes: Array<{
  id: AppTheme
  label: string
  bg: string
  side: string
  border: string
  accent: string
}> = [
  { id: 'warm', label: '暖纸白', bg: '#faf8f2', side: '#f3efe6', border: '#d8d0c0', accent: '#c96442' },
  { id: 'light', label: '纯净白', bg: '#ffffff', side: '#f6f7f9', border: '#dee2e6', accent: '#2563eb' },
  { id: 'dark', label: '深邃暗', bg: '#141416', side: '#1b1b1e', border: '#34343d', accent: '#3b82f6' }
]

const engines: Array<{
  id: AgentEngine
  label: string
  presets: string[]
  pathHint: string
}> = [
  {
    id: 'agy',
    label: 'AGY',
    presets: ['', 'gemini-2.5-pro', 'gemini-2.5-flash', 'claude-sonnet-4'],
    pathHint: '例如 C:\\Users\\...\\AppData\\Local\\agy\\bin\\agy.exe'
  },
  {
    id: 'claude',
    label: 'Claude Code',
    presets: ['', 'sonnet', 'opus', 'haiku'],
    pathHint: '例如 C:\\Users\\...\\.local\\bin\\claude.exe'
  },
  {
    id: 'codex',
    label: 'Codex',
    presets: ['', 'gpt-5.4', 'gpt-5.3-codex', 'o3'],
    pathHint: '例如 codex 或绝对路径'
  }
]

function engineTitle(id: string) {
  if (id === 'claude') return 'Claude Code'
  if (id === 'codex') return 'Codex'
  return 'AGY'
}

function livePresetsFor(eng: AgentEngine) {
  const live = store.engineModels[eng] || []
  if (live.length > 0) return live.slice(0, 8)
  const fallback = engines.find((e) => e.id === eng)?.presets.filter(Boolean) || []
  return fallback.map((id) => ({ id, label: id }))
}

function setModel(eng: AgentEngine, val: string) {
  store.updateSettings({
    models: { ...store.settings.models, [eng]: val }
  })
}

function onModelInput(eng: AgentEngine, e: Event) {
  setModel(eng, (e.target as HTMLInputElement).value)
}

function onPathInput(eng: AgentEngine, e: Event) {
  store.updateSettings({
    cliPaths: { ...store.settings.cliPaths, [eng]: (e.target as HTMLInputElement).value }
  })
}

function onPersonalPromptInput(e: Event) {
  store.updateSettings({
    personalWritingPrompt: (e.target as HTMLTextAreaElement).value
  })
}

function togglePersonalPrompt() {
  store.updateSettings({
    enablePersonalWritingPrompt: !store.settings.enablePersonalWritingPrompt
  })
}

function resetPersonalPrompt() {
  store.updateSettings({
    enablePersonalWritingPrompt: true,
    personalWritingPrompt: DEFAULT_PERSONAL_WRITING_PROMPT
  })
  store.showToast('已恢复默认个人写作偏好')
}

async function runCheckCli() {
  void Promise.all([
    store.fetchEngineModels('agy'),
    store.fetchEngineModels('claude'),
    store.fetchEngineModels('codex')
  ])
  if (!window.mark5.checkCliStatus) return
  checkingCli.value = true
  try {
    cliStatuses.value = await window.mark5.checkCliStatus({
      agyPath: store.settings.cliPaths.agy,
      claudePath: store.settings.cliPaths.claude,
      codexPath: store.settings.cliPaths.codex
    })
  } finally {
    checkingCli.value = false
  }
}

const onEsc = (e: KeyboardEvent) => {
  if (e.key === 'Escape') emit('close')
}

onMounted(() => {
  window.addEventListener('keydown', onEsc)
  if (tab.value === 'backend') {
    void runCheckCli()
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onEsc)
})
</script>

<style scoped>
.settings-mask {
  position: fixed;
  inset: 0;
  background: rgba(20, 20, 22, 0.36);
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
}
.settings-dialog {
  width: 660px;
  max-width: 92vw;
  height: 520px;
  max-height: 86vh;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 16px;
  box-shadow: var(--shadow-pop);
  display: flex;
  overflow: hidden;
}
.settings-nav {
  width: 176px;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-hairline);
  padding: 16px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
}
.settings-brand {
  font-size: 13px;
  font-weight: 600;
  color: var(--ink-muted);
  padding: 4px 10px 10px;
}
.snav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 8px;
  font-size: 13px;
  color: var(--ink-body);
  text-align: left;
}
.snav-item:hover {
  background: var(--bg-hover);
}
.snav-item.active {
  background: var(--bg-active);
  color: var(--ink-primary);
  font-weight: 500;
}
.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.settings-head {
  height: 48px;
  padding: 0 16px 0 20px;
  border-bottom: 1px solid var(--border-hairline);
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
  font-weight: 600;
  color: var(--ink-primary);
  flex-shrink: 0;
}
.icon-close {
  width: 28px;
  height: 28px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-muted);
}
.settings-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.s-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.s-row-between {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.s-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--ink-primary);
}
.s-sub-label {
  font-size: 11.5px;
  color: var(--ink-muted);
  margin-top: 4px;
}
.theme-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.theme-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  padding: 10px;
  border-radius: 10px;
  border: 1.5px solid var(--border-hairline);
  background: var(--bg-sidebar);
}
.theme-card.active {
  border-color: var(--accent);
}
.theme-swatch {
  width: 100%;
  height: 44px;
  border-radius: 7px;
  border: 1px solid;
  position: relative;
  overflow: hidden;
}
.swatch-side {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 30%;
}
.swatch-dot {
  position: absolute;
  right: 8px;
  bottom: 8px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.theme-name {
  font-size: 12px;
  color: var(--ink-body);
}
.seg-row {
  display: flex;
  gap: 6px;
}
.seg-btn {
  flex: 1;
  padding: 6px 10px;
  border-radius: 8px;
  border: 1px solid var(--border-hairline);
  background: var(--bg-sidebar);
  font-size: 12.5px;
  color: var(--ink-body);
}
.seg-btn.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--ink-primary);
  font-weight: 500;
}
.engine-card {
  padding: 12px 14px;
  border: 1px solid var(--border-hairline);
  border-radius: 10px;
  background: var(--bg-sidebar);
}
.s-input {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  background: var(--bg-canvas);
  color: var(--ink-primary);
  font-size: 12.5px;
  outline: none;
}
.s-input:focus {
  border-color: var(--accent);
}
.preset-row {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.preset-chip {
  font-size: 11.5px;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--bg-subtle);
  color: var(--ink-muted);
}
.preset-chip.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 500;
}
.btn-mini {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  padding: 4px 9px;
  border-radius: 7px;
  border: 1px solid var(--border-soft);
  background: var(--bg-canvas);
  color: var(--ink-body);
}
.btn-mini:hover:not(:disabled) {
  background: var(--bg-subtle);
}
.btn-mini-active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 500;
}
.writing-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.s-textarea {
  width: 100%;
  min-height: 180px;
  padding: 10px 12px;
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  background: var(--bg-canvas);
  color: var(--ink-primary);
  font-family: inherit;
  font-size: 12.5px;
  line-height: 1.65;
  resize: vertical;
  outline: none;
}
.s-textarea:focus {
  border-color: var(--accent);
}
.s-textarea.dimmed {
  opacity: 0.55;
}
.cli-status-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-top: 6px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-sidebar);
  border: 1px solid var(--border-hairline);
}
.cli-status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.cli-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #dc2626;
  flex-shrink: 0;
}
.cli-dot.ok {
  background: var(--success);
}
.cli-name {
  font-weight: 500;
  color: var(--ink-primary);
  width: 88px;
}
.cli-ver {
  color: var(--ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spin {
  animation: spinAnim 0.9s linear infinite;
}
@keyframes spinAnim {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
