<template>
  <div class="app-root">
    <!-- Clean Welcome Launcher when no workspace is open yet -->
    <WelcomeView
      v-if="!store.activeWorkspace"
      @open-settings="openSettings('appearance')"
    />

    <template v-else>
      <!-- Left Sidebar (Docs Directory/TOC in 'docs' mode, Session Manager in 'collab' mode) -->
      <LeftSidebar
        v-if="showSidebar"
        :mode="mode"
        @update:mode="switchMode"
        @collapse="showSidebar = false"
        @new-session="handleNewSession"
        @open-settings="openSettings('appearance')"
      />

      <!-- Middle Chat Pane (only in 'collab' AI writing mode) -->
      <ChatPane
        v-if="mode === 'collab'"
        :sidebar-visible="showSidebar"
        :doc-visible="showDoc"
        :pane-width="chatWidth"
        @expand-sidebar="showSidebar = true"
        @open-doc="showDoc = true"
        @open-model-settings="openSettings('backend')"
      />

      <!-- Draggable Resizer between ChatPane and MuyaEditorPane (P1-6) -->
      <div
        v-if="mode === 'collab' && showDoc"
        class="pane-resizer"
        :class="{ dragging: isDragging }"
        title="拖动调节对话栏宽度"
        @mousedown="startResize"
      />

      <!-- Right Muya WYSIWYG Document Pane (always visible in 'files'/'docs' mode, toggleable in 'collab' mode) -->
      <MuyaEditorPane
        v-show="mode !== 'collab' || showDoc"
        :sidebar-visible="showSidebar"
        :can-close="mode === 'collab'"
        @expand-sidebar="showSidebar = true"
        @close-doc="showDoc = false"
        @open-history="showHistory = true"
      />
    </template>

    <!-- Minimal History Modal -->
    <div v-if="showHistory" class="modal-mask" @click.self="showHistory = false">
      <div class="modal-box">
        <div class="modal-head">
          <span>版本历史</span>
          <button @click="showHistory = false">✕</button>
        </div>
        <div class="modal-body">
          <div v-if="!store.history.length" class="empty-history">暂无历史版本</div>
          <div v-for="snap in store.history" :key="snap.id" class="snap-card">
            <div class="snap-row">
              <span class="snap-title">{{ snap.summary }}</span>
              <span class="snap-time">{{ new Date(snap.timestamp).toLocaleTimeString() }}</span>
            </div>
            <button
              class="btn-outline"
              @click="store.rollbackToSnapshot(snap); showHistory = false"
            >
              恢复此版本
            </button>
          </div>
        </div>
      </div>
    </div>

    <SettingsModal
      v-if="showSettings"
      :initial-tab="settingsTab"
      @close="showSettings = false"
    />

    <div v-if="store.toastMessage" class="toast">{{ store.toastMessage }}</div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useAppStore } from '@/stores/appStore'
import LeftSidebar from '@/components/LeftSidebar.vue'
import ChatPane from '@/components/ChatPane.vue'
import MuyaEditorPane from '@/components/MuyaEditorPane.vue'
import SettingsModal from '@/components/SettingsModal.vue'
import WelcomeView from '@/components/WelcomeView.vue'

const store = useAppStore()
const mode = ref<'files' | 'docs' | 'collab'>('docs')
const showSidebar = ref(true)
const showDoc = ref(true)
const showHistory = ref(false)
const showSettings = ref(false)
const settingsTab = ref<'appearance' | 'backend'>('appearance')

function openSettings(tab: 'appearance' | 'backend') {
  settingsTab.value = tab
  showSettings.value = true
}

const savedWidth = Number(localStorage.getItem('mc_chat_width') || 390)
const chatWidth = ref(Number.isFinite(savedWidth) ? Math.min(560, Math.max(300, savedWidth)) : 390)
const isDragging = ref(false)

function startResize(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
  const startX = e.clientX
  const startW = chatWidth.value

  const onMove = (ev: MouseEvent) => {
    const next = Math.min(560, Math.max(300, startW + (ev.clientX - startX)))
    chatWidth.value = Math.round(next)
  }
  const onUp = () => {
    isDragging.value = false
    localStorage.setItem('mc_chat_width', String(chatWidth.value))
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

function switchMode(next: 'files' | 'docs' | 'collab') {
  mode.value = next
  if (next !== 'collab') {
    showDoc.value = true
  } else if (window.innerWidth < 1180) {
    // Auto-collapse sidebar on narrow screens so chat + doc stay readable (P1-6)
    showSidebar.value = false
  }
}

async function handleNewSession() {
  mode.value = 'collab'
  await store.createSession('新会话', store.preferredEngine)
}

watch(
  () => store.chatDraftRequest.seq,
  () => {
    if (store.chatDraftRequest.text) {
      mode.value = 'collab'
    }
  }
)

const onWindowKeydown = (e: KeyboardEvent) => {
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'n') {
    e.preventDefault()
    void window.mark5.openNewWindow?.()
  } else if ((e.ctrlKey || e.metaKey) && e.key === ',') {
    e.preventDefault()
    openSettings('appearance')
  }
}

onMounted(() => {
  window.addEventListener('keydown', onWindowKeydown)
  store.init()
})

onUnmounted(() => {
  window.removeEventListener('keydown', onWindowKeydown)
})
</script>

<style scoped>
.app-root {
  display: flex;
  height: 100vh;
  width: 100vw;
  background: var(--bg-canvas);
  overflow: hidden;
}
.pane-resizer {
  width: 5px;
  margin-left: -3px;
  margin-right: -2px;
  cursor: col-resize;
  z-index: 15;
  flex-shrink: 0;
  position: relative;
}
.pane-resizer::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 2px;
  width: 1px;
  background: transparent;
  transition: background 0.15s ease;
}
.pane-resizer:hover::after,
.pane-resizer.dragging::after {
  background: var(--accent);
}
.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(26, 25, 24, 0.25);
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
}
.modal-box {
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  width: 420px;
  max-width: 92vw;
  box-shadow: var(--shadow-pop);
}
.modal-head {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-hairline);
  font-weight: 600;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.modal-body {
  padding: 14px 16px;
  max-height: 70vh;
  overflow-y: auto;
}
.empty-history {
  color: var(--ink-muted);
  text-align: center;
  padding: 24px 0;
}
.snap-card {
  padding: 10px 12px;
  border: 1px solid var(--border-hairline);
  border-radius: 10px;
  margin-bottom: 8px;
}
.snap-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.snap-title {
  font-weight: 500;
}
.snap-time {
  font-size: 11.5px;
  color: var(--ink-muted);
}
.toast {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 80;
  background: var(--ink-primary);
  color: #ffffff;
  padding: 8px 14px;
  border-radius: 10px;
  font-size: 12.5px;
  box-shadow: var(--shadow-pop);
}
</style>
