<template>
  <aside class="left-sidebar">
    <!-- Brand Header -->
    <div class="sidebar-top">
      <span class="brand-word">Mark 5</span>
      <div class="sidebar-top-actions">
        <button class="icon-btn" title="新建窗口 (Ctrl+Shift+N)" @click="openNewWindow">
          <SquarePlus :size="15" :stroke-width="1.75" />
        </button>
        <button class="icon-btn" title="收起侧栏" @click="$emit('collapse')">
          <PanelLeftClose :size="16" :stroke-width="1.75" />
        </button>
      </div>
    </div>

    <!-- 3-Mode Switcher: IDE File Tree | MD Docs + TOC | AI Sessions -->
    <div class="mode-switch-wrap">
      <div class="mode-switch" role="tablist">
        <button
          class="mode-tab"
          :class="{ active: mode === 'files' }"
          role="tab"
          :aria-selected="mode === 'files'"
          @click="$emit('update:mode', 'files')"
        >
          文件
        </button>
        <button
          class="mode-tab"
          :class="{ active: mode === 'docs' }"
          role="tab"
          :aria-selected="mode === 'docs'"
          @click="$emit('update:mode', 'docs')"
        >
          文档
        </button>
        <button
          class="mode-tab"
          :class="{ active: mode === 'collab' }"
          role="tab"
          :aria-selected="mode === 'collab'"
          @click="$emit('update:mode', 'collab')"
        >
          <span>对话</span>
          <span v-if="hasAnyRunning" class="mode-dot running" />
          <span v-else-if="hasAnyUnread" class="mode-dot unread" />
        </button>
      </div>
    </div>

    <!-- Primary Nav Actions -->
    <nav class="primary-nav">
      <button v-if="mode !== 'collab'" class="nav-row" @click="createNewDoc">
        <Plus :size="15" :stroke-width="1.85" />
        <span>新建文档</span>
      </button>

      <button v-else class="nav-row" @click="$emit('new-session')">
        <Plus :size="15" :stroke-width="1.85" />
        <span>新建会话</span>
      </button>

      <button class="nav-row" :class="{ active: showSearch }" @click="toggleSearch">
        <Search :size="15" :stroke-width="1.75" />
        <span>搜索</span>
      </button>

      <div v-if="showSearch" class="search-wrap">
        <input
          ref="searchInputEl"
          v-model="query"
          class="search-input"
          :placeholder="mode === 'collab' ? '搜索会话...' : mode === 'files' ? '搜索全部文件...' : '搜索 Markdown 文档...'"
        />
      </div>
    </nav>

    <!-- Scrollable List -->
    <div class="sidebar-scroll">
      <template v-if="mode === 'collab'">
        <!-- Pinned Section -->
        <template v-if="pinnedSessions.length">
          <button class="section-toggle" @click="pinnedOpen = !pinnedOpen">
            <span>Pinned</span>
            <span class="chev">{{ pinnedOpen ? '▾' : '▸' }}</span>
          </button>
          <template v-if="pinnedOpen">
            <div
              v-for="s in pinnedSessions"
              :key="s.id"
              class="list-item"
              :class="{
                active: s.id === store.activeSessionId,
                linked: s.docRelPath && s.docRelPath === store.activeDocRelPath
              }"
              @click="store.selectSession(s.id)"
            >
              <div class="item-main">
                <div class="item-top-row">
                  <Loader2
                    v-if="store.sessionBusyMap[s.id]"
                    :size="13"
                    :stroke-width="2.1"
                    class="state-spinner"
                    :title="store.sessionLiveStatusMap[s.id] || '思考中...'"
                  />
                  <span v-else-if="s.unread" class="state-dot unread" title="已完成（未读）" />
                  <input
                    v-if="renamingId === s.id"
                    v-model="renameDraft"
                    class="rename-input"
                    @click.stop
                    @keydown.enter="commitRename(s.id)"
                    @keydown.esc="renamingId = null"
                    @blur="commitRename(s.id)"
                  />
                  <span v-else class="item-title">{{ s.title }}</span>
                </div>
              </div>
              <div class="item-right">
                <FileText
                  v-if="s.docRelPath"
                  class="doc-mini-icon"
                  :size="13"
                  :stroke-width="1.75"
                />
                <span class="item-time">{{ formatTime(s.updatedAt) }}</span>
                <button class="more-dots" title="更多操作" @click.stop="openMenu(s.id, $event)">
                  ···
                </button>
              </div>
            </div>
          </template>
        </template>

        <!-- Recents Section -->
        <button class="section-toggle" @click="recentsOpen = !recentsOpen">
          <span>最近会话</span>
          <span class="chev">{{ recentsOpen ? '▾' : '▸' }}</span>
        </button>
        <template v-if="recentsOpen">
          <div
            v-for="s in recentSessions"
            :key="s.id"
            class="list-item"
            :class="{
              active: s.id === store.activeSessionId,
              linked: s.docRelPath && s.docRelPath === store.activeDocRelPath
            }"
            @click="store.selectSession(s.id)"
          >
            <div class="item-main">
              <div class="item-top-row">
                <Loader2
                  v-if="store.sessionBusyMap[s.id]"
                  :size="13"
                  :stroke-width="2.1"
                  class="state-spinner"
                  :title="store.sessionLiveStatusMap[s.id] || '思考中...'"
                />
                <span v-else-if="s.unread" class="state-dot unread" title="已完成（未读）" />
                <input
                  v-if="renamingId === s.id"
                  v-model="renameDraft"
                  class="rename-input"
                  @click.stop
                  @keydown.enter="commitRename(s.id)"
                  @keydown.esc="renamingId = null"
                  @blur="commitRename(s.id)"
                />
                <span v-else class="item-title">{{ s.title }}</span>
              </div>
            </div>
            <div class="item-right">
              <svg
                v-if="s.docRelPath"
                class="doc-mini-icon"
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.8"
                :title="s.docRelPath"
              >
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <span class="item-time">{{ formatTime(s.updatedAt) }}</span>
              <button class="more-dots" title="更多操作" @click.stop="openMenu(s.id, $event)">
                ···
              </button>
            </div>
          </div>
          <div v-if="!recentSessions.length && !pinnedSessions.length" class="empty-muted">
            暂无会话
          </div>
        </template>

        <!-- Archived Collapsible Section -->
        <template v-if="archivedSessions.length">
          <button class="section-toggle" style="margin-top: 10px" @click="archivedOpen = !archivedOpen">
            <span>已归档 ({{ archivedSessions.length }})</span>
            <span class="chev">{{ archivedOpen ? '▾' : '▸' }}</span>
          </button>
          <template v-if="archivedOpen">
            <div
              v-for="s in archivedSessions"
              :key="s.id"
              class="list-item archived-row"
              :class="{ active: s.id === store.activeSessionId }"
              @click="store.selectSession(s.id)"
            >
              <input
                v-if="renamingId === s.id"
                v-model="renameDraft"
                class="rename-input"
                @click.stop
                @keydown.enter="commitRename(s.id)"
                @keydown.esc="renamingId = null"
                @blur="commitRename(s.id)"
              />
              <span v-else class="item-title">{{ s.title }}</span>
              <div class="item-right">
                <span class="item-time">{{ formatTime(s.updatedAt) }}</span>
                <button class="more-dots" @click.stop="openMenu(s.id, $event)">···</button>
              </div>
            </div>
          </template>
        </template>
      </template>

      <template v-else-if="mode === 'files'">
        <div class="section-caption">工作区文件树</div>
        <FileTreeNode
          v-for="entry in filteredAllFileTree"
          :key="entry.relPath"
          :entry="entry"
          :depth="0"
          :active-path="store.activeDocRelPath"
          :force-open="Boolean(query.trim())"
          :renaming-path="renamingFilePath"
          :rename-draft="fileRenameDraft"
          @select="store.openDocument($event)"
          @open-menu="openFileMenu"
          @update:rename-draft="fileRenameDraft = $event"
          @commit-rename="commitFileRename"
          @cancel-rename="cancelFileRename"
        />
        <div v-if="!filteredAllFileTree.length" class="empty-muted">当前工作区暂无文件</div>
      </template>

      <template v-else>
        <div class="section-caption">Markdown 文档</div>
        <div
          v-for="f in filteredMdFiles"
          :key="f.relPath"
          class="doc-list-item"
          :class="{ active: f.relPath === store.activeDocRelPath }"
          :title="f.relPath"
          @click="store.openDocument(f.relPath)"
          @contextmenu.prevent.stop="openFileMenu(f, $event)"
        >
          <FileText :size="14" :stroke-width="1.75" class="doc-list-icon" />
          <input
            v-if="renamingFilePath === f.relPath"
            v-model="fileRenameDraft"
            class="rename-input"
            @click.stop
            @keydown.enter.stop="commitFileRename"
            @keydown.esc.stop="cancelFileRename"
            @blur="commitFileRename"
          />
          <span v-else class="item-title">{{ f.relPath }}</span>
          <button
            v-if="renamingFilePath !== f.relPath"
            class="doc-more-dots"
            title="更多操作"
            @click.stop="openFileMenu(f, $event)"
          >
            ···
          </button>
        </div>
        <div v-if="!filteredMdFiles.length" class="empty-muted">当前目录暂无 Markdown 文档</div>

        <template v-if="store.toc.length">
          <div class="section-caption" style="margin-top: 14px">大纲</div>
          <button
            v-for="(h, i) in store.toc"
            :key="`${i}-${h.slug}`"
            class="toc-item"
            :style="{ paddingLeft: `${(h.lvl - 1) * 10 + 10}px` }"
            @click="store.requestScrollToHeading(h.content, i)"
          >
            {{ h.content }}
          </button>
        </template>
      </template>
    </div>

    <!-- Claude-Style Hover Context Menu -->
    <div
      v-if="menuSession"
      class="session-menu"
      :style="{ top: menuPos.top + 'px', left: menuPos.left + 'px' }"
      @click.stop
    >
      <button
        v-if="store.sessionBusyMap[menuSession.id]"
        class="smenu-item"
        @click="onMenuStop(menuSession.id)"
      >
        <span>停止运行</span>
      </button>
      <button class="smenu-item" @click="onMenuPin(menuSession.id)">
        <span>{{ menuSession.pinned ? '取消置顶' : '置顶' }}</span>
        <span class="kbd">P</span>
      </button>
      <button class="smenu-item" @click="onMenuRename(menuSession.id, menuSession.title)">
        <span>重命名</span>
        <span class="kbd">R</span>
      </button>
      <div class="smenu-sep" />
      <button class="smenu-item" @click="onMenuArchive(menuSession.id)">
        <span>{{ menuSession.archived ? '取消归档' : '归档' }}</span>
        <span class="kbd">A</span>
      </button>
      <button class="smenu-item danger" @click="onMenuDelete(menuSession.id)">
        <span>删除</span>
        <span class="kbd">D</span>
      </button>
    </div>

    <!-- Claude-Style Floating File / Folder Context Menu -->
    <div
      v-if="fileMenuEntry"
      class="session-menu file-context-menu"
      :style="{ top: fileMenuPos.top + 'px', left: fileMenuPos.left + 'px' }"
      @click.stop
    >
      <template v-if="fileMenuEntry.isDir">
        <button class="smenu-item" @click="onFileNewDoc(fileMenuEntry)">
          <span>新建 Markdown</span>
          <span class="kbd">N</span>
        </button>
        <button class="smenu-item" @click="onFileNewFolder(fileMenuEntry)">
          <span>新建子文件夹</span>
        </button>
        <div class="smenu-sep" />
      </template>
      <button class="smenu-item" @click="onFileStartRename(fileMenuEntry)">
        <span>重命名</span>
        <span class="kbd">R</span>
      </button>
      <button v-if="!fileMenuEntry.isDir" class="smenu-item" @click="onFileDuplicate(fileMenuEntry)">
        <span>创建副本</span>
      </button>
      <button class="smenu-item" @click="onFileCopyPath(fileMenuEntry)">
        <span>复制路径</span>
        <span class="kbd">C</span>
      </button>
      <button class="smenu-item" @click="onFileReveal(fileMenuEntry)">
        <span>在资源管理器中显示</span>
      </button>
      <div class="smenu-sep" />
      <button class="smenu-item danger" @click="onFileDelete(fileMenuEntry)">
        <span>{{ fileMenuEntry.isDir ? '删除文件夹' : '删除' }}</span>
        <span class="kbd">D</span>
      </button>
    </div>

    <!-- Bottom Workspace Footer -->
    <div class="sidebar-footer">
      <select
        :value="store.activeWorkspace?.rootPath"
        :title="store.activeWorkspace?.rootPath"
        class="ws-select"
        @change="onSelectWorkspace"
      >
        <option v-for="w in store.workspaces" :key="w.rootPath" :value="w.rootPath">
          {{ w.name }}
        </option>
      </select>
      <button class="icon-btn" title="打开工作区目录" @click="store.pickWorkspace()">
        <FolderPlus :size="15" :stroke-width="1.75" />
      </button>
      <button class="icon-btn" title="设置 (Ctrl+,)" @click="$emit('open-settings')">
        <Settings :size="15" :stroke-width="1.75" />
      </button>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from 'vue'
import { FileText, FolderPlus, Loader2, PanelLeftClose, Plus, Search, Settings, SquarePlus } from 'lucide-vue-next'
import type { FileEntry } from '@shared/types'
import { useAppStore } from '@/stores/appStore'
import FileTreeNode from './FileTreeNode.vue'

defineProps<{
  mode: 'files' | 'docs' | 'collab'
}>()

defineEmits<{
  (e: 'update:mode', mode: 'files' | 'docs' | 'collab'): void
  (e: 'collapse'): void
  (e: 'new-session'): void
  (e: 'open-settings'): void
}>()

const store = useAppStore()
const showSearch = ref(false)
const query = ref('')
const searchInputEl = ref<HTMLInputElement | null>(null)

const hasAnyRunning = computed(() =>
  Object.values(store.sessionBusyMap).some(Boolean)
)
const hasAnyUnread = computed(() =>
  store.sessions.some((s) => !s.archived && s.unread)
)

const pinnedOpen = ref(true)
const recentsOpen = ref(true)
const archivedOpen = ref(false)

const menuSessionId = ref<string | null>(null)
const menuPos = reactive({ top: 100, left: 120 })
const renamingId = ref<string | null>(null)
const renameDraft = ref('')

const fileMenuEntry = ref<FileEntry | null>(null)
const fileMenuPos = reactive({ top: 100, left: 120 })
const renamingFilePath = ref<string | null>(null)
const fileRenameDraft = ref('')

const menuSession = computed(
  () => store.sessions.find((s) => s.id === menuSessionId.value) ?? null
)

async function toggleSearch() {
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    await nextTick()
    searchInputEl.value?.focus()
  } else {
    query.value = ''
  }
}

function filterTree(entries: FileEntry[], q: string): FileEntry[] {
  if (!q) return entries
  const out: FileEntry[] = []
  for (const item of entries) {
    if (item.isDir) {
      const matchedChildren = filterTree(item.children || [], q)
      if (matchedChildren.length > 0 || item.name.toLowerCase().includes(q)) {
        out.push({ ...item, children: matchedChildren.length ? matchedChildren : item.children })
      }
    } else if (item.relPath.toLowerCase().includes(q) || item.name.toLowerCase().includes(q)) {
      out.push(item)
    }
  }
  return out
}

const filteredAllFileTree = computed(() =>
  filterTree(store.allFiles, query.value.trim().toLowerCase())
)

const flatMdFiles = computed(() => {
  const out: FileEntry[] = []
  const walk = (list: FileEntry[]) => {
    for (const item of list) {
      if (item.isDir && item.children) walk(item.children)
      else if (!item.isDir) out.push(item)
    }
  }
  walk(store.files)
  return out
})

const filteredMdFiles = computed(() => {
  const q = query.value.trim().toLowerCase()
  return flatMdFiles.value.filter((f) => !q || f.relPath.toLowerCase().includes(q))
})

const filteredSessions = computed(() => {
  const q = query.value.trim().toLowerCase()
  return store.sessions.filter((s) => !q || s.title.toLowerCase().includes(q))
})

const pinnedSessions = computed(() =>
  filteredSessions.value.filter((s) => s.pinned && !s.archived)
)
const recentSessions = computed(() =>
  filteredSessions.value.filter((s) => !s.pinned && !s.archived)
)
const archivedSessions = computed(() => filteredSessions.value.filter((s) => s.archived))

function openMenu(sessionId: string, e: MouseEvent) {
  menuSessionId.value = sessionId
  menuPos.top = Math.min(window.innerHeight - 210, e.clientY + 6)
  menuPos.left = Math.min(window.innerWidth - 190, Math.max(24, e.clientX - 40))
}

async function onMenuStop(id: string) {
  menuSessionId.value = null
  await store.stopSession(id)
}

async function onMenuPin(id: string) {
  menuSessionId.value = null
  await store.togglePinSession(id)
}

async function onMenuRename(id: string, title: string) {
  menuSessionId.value = null
  renamingId.value = id
  renameDraft.value = title
  await nextTick()
  const inp = document.querySelector('.rename-input') as HTMLInputElement | null
  inp?.focus()
  inp?.select()
}

async function commitRename(id: string) {
  if (renamingId.value !== id) return
  renamingId.value = null
  await store.renameSession(id, renameDraft.value)
}

async function onMenuArchive(id: string) {
  menuSessionId.value = null
  await store.toggleArchiveSession(id)
}

async function onMenuDelete(id: string) {
  menuSessionId.value = null
  await store.deleteSession(id)
}

function onSelectWorkspace(e: Event) {
  const val = (e.target as HTMLSelectElement).value
  const ws = store.workspaces.find((w) => w.rootPath === val)
  if (ws) store.switchWorkspace(ws)
}

function formatTime(ts: number): string {
  const diff = Date.now() - ts
  if (diff < 60_000) return '刚刚'
  const d = new Date(ts)
  const today = new Date()
  if (d.toDateString() === today.toDateString()) {
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  }
  return `${d.getMonth() + 1}-${d.getDate()}`
}

function openFileMenu(entry: FileEntry, e: MouseEvent) {
  menuSessionId.value = null
  const target = e.currentTarget as HTMLElement | null
  const rect = target?.getBoundingClientRect()
  const anchorY = e.type === 'contextmenu' ? e.clientY : (rect ? rect.bottom + 4 : e.clientY)
  const anchorX = e.type === 'contextmenu' ? e.clientX : (rect ? rect.right - 140 : e.clientX)
  fileMenuPos.top = Math.min(window.innerHeight - 280, Math.max(48, anchorY))
  fileMenuPos.left = Math.min(window.innerWidth - 216, Math.max(24, anchorX))
  fileMenuEntry.value = entry
}

async function onFileStartRename(entry: FileEntry) {
  fileMenuEntry.value = null
  renamingFilePath.value = entry.relPath
  fileRenameDraft.value = entry.name
  await nextTick()
  const inp = (document.querySelector('.tree-rename-input') ||
    document.querySelector('.doc-list-item .rename-input')) as HTMLInputElement | null
  if (inp) {
    inp.focus()
    const dot = !entry.isDir ? entry.name.lastIndexOf('.') : -1
    if (dot > 0) inp.setSelectionRange(0, dot)
    else inp.select()
  }
}

async function commitFileRename() {
  const oldPath = renamingFilePath.value
  if (!oldPath) return
  renamingFilePath.value = null
  const nextName = fileRenameDraft.value.trim()
  if (!nextName) return
  await store.renameFileOrDir(oldPath, nextName)
}

function cancelFileRename() {
  renamingFilePath.value = null
}

async function onFileDuplicate(entry: FileEntry) {
  fileMenuEntry.value = null
  await store.duplicateWorkspaceFile(entry.relPath)
}

async function onFileCopyPath(entry: FileEntry) {
  fileMenuEntry.value = null
  try {
    await navigator.clipboard.writeText(entry.relPath)
    store.showToast(`已复制路径：${entry.relPath}`)
  } catch {
    store.showToast('复制失败')
  }
}

async function onFileReveal(entry: FileEntry) {
  fileMenuEntry.value = null
  await store.revealInExplorer(entry.relPath)
}

async function onFileDelete(entry: FileEntry) {
  fileMenuEntry.value = null
  const label = entry.isDir ? `文件夹「${entry.name}」及其全部内容` : `文件「${entry.name}」`
  if (!window.confirm(`确定要永久删除${label}吗？`)) return
  await store.deleteFileOrDir(entry.relPath)
}

async function onFileNewDoc(entry: FileEntry) {
  fileMenuEntry.value = null
  const created = await store.createDocInFolder(entry.relPath)
  if (created) {
    const name = created.split('/').pop() || created
    await onFileStartRename({ name, relPath: created, isDir: false })
  }
}

async function onFileNewFolder(entry: FileEntry) {
  fileMenuEntry.value = null
  const folderName = window.prompt(`在「${entry.name}」下新建子文件夹名称：`)
  if (!folderName || !folderName.trim()) return
  await store.createSubFolder(entry.relPath, folderName.trim())
}

const closeMenu = () => {
  menuSessionId.value = null
  fileMenuEntry.value = null
}

const onKeyShortcut = (e: KeyboardEvent) => {
  if (!menuSession.value && !fileMenuEntry.value) return
  if (e.ctrlKey || e.metaKey || e.altKey) return
  if (
    e.target instanceof HTMLElement &&
    (['INPUT', 'TEXTAREA'].includes(e.target.tagName) || e.target.isContentEditable)
  ) {
    return
  }
  const key = e.key.toLowerCase()
  if (key === 'escape') {
    menuSessionId.value = null
    fileMenuEntry.value = null
    return
  }
  if (fileMenuEntry.value) {
    const entry = fileMenuEntry.value
    if (key === 'r') {
      e.preventDefault()
      void onFileStartRename(entry)
    } else if (key === 'c') {
      e.preventDefault()
      void onFileCopyPath(entry)
    } else if (key === 'd') {
      e.preventDefault()
      void onFileDelete(entry)
    } else if (key === 'n' && entry.isDir) {
      e.preventDefault()
      void onFileNewDoc(entry)
    }
    return
  }
  const s = menuSession.value!
  if (key === 'p') {
    e.preventDefault()
    onMenuPin(s.id)
  } else if (key === 'r') {
    e.preventDefault()
    onMenuRename(s.id, s.title)
  } else if (key === 'a') {
    e.preventDefault()
    onMenuArchive(s.id)
  } else if (key === 'd') {
    e.preventDefault()
    onMenuDelete(s.id)
  }
}

async function openNewWindow() {
  await window.mark5.openNewWindow?.()
}

async function createNewDoc() {
  if (!store.activeWorkspace) return
  const relPath = `未命名-${Date.now().toString().slice(-4)}.md`
  const today = new Date().toISOString().slice(0, 10)
  const initial = `# 未命名文档\n\n> ${today} · 你\n\n`
  await window.mark5.writeMarkdownFile(store.activeWorkspace.rootPath, relPath, initial)
  await store.refreshFiles()
  await store.openDocument(relPath)
}

onMounted(() => {
  document.addEventListener('click', closeMenu)
  document.addEventListener('keydown', onKeyShortcut)
})

onUnmounted(() => {
  document.removeEventListener('click', closeMenu)
  document.removeEventListener('keydown', onKeyShortcut)
})
</script>

<style scoped>
.left-sidebar {
  width: 244px;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-hairline);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  user-select: none;
}
.sidebar-top {
  height: 46px;
  padding: 0 12px 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.sidebar-top-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.brand-word {
  font-size: 15.5px;
  font-weight: 650;
  letter-spacing: -0.02em;
  color: var(--ink-primary);
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
.mode-switch-wrap {
  padding: 2px 12px 8px;
}
.mode-switch {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2px;
  padding: 3px;
  background: var(--bg-subtle);
  border-radius: 9px;
}
.mode-tab {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 5px 8px;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink-muted);
  border-radius: 7px;
  background: transparent;
}
.mode-tab:hover {
  color: var(--ink-primary);
  background: transparent;
}
.mode-tab.active {
  background: var(--bg-canvas);
  color: var(--ink-primary);
  box-shadow: 0 1px 2px rgba(26, 25, 24, 0.08);
}
.mode-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.mode-dot.running {
  background: var(--success);
  animation: dotPulse 1.2s infinite;
}
.mode-dot.unread {
  background: #2563eb;
}
.primary-nav {
  padding: 4px 10px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-row {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 7px 10px;
  font-size: 13px;
  font-weight: 500;
  color: var(--ink-body);
  text-align: left;
  border-radius: 8px;
}
.nav-row:hover {
  background: var(--bg-hover);
}
.nav-row.active {
  background: var(--bg-active);
  color: var(--ink-primary);
}
.search-wrap {
  padding: 4px 4px 6px;
}
.search-input {
  width: 100%;
  padding: 6px 9px;
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  background: var(--bg-canvas);
  outline: none;
}
.sidebar-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 4px 10px 12px;
}
.section-toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11.5px;
  font-weight: 500;
  color: var(--ink-muted);
  padding: 6px 8px 4px;
  border-radius: 6px;
}
.section-toggle:hover {
  background: transparent;
  color: var(--ink-primary);
}
.chev {
  font-size: 10px;
}
.section-caption-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px 4px 10px;
}
.caption-add-btn {
  font-size: 11.5px;
  color: var(--ink-muted);
  padding: 2px 6px;
  border-radius: 5px;
}
.caption-add-btn:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.section-caption {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--ink-muted);
  padding: 6px 10px 4px;
}
.list-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 7px 8px 7px 10px;
  border-radius: 8px;
  cursor: pointer;
  color: var(--ink-body);
  font-size: 13px;
  margin-bottom: 1px;
  transition: background 0.12s ease;
}
.list-item:hover {
  background: var(--bg-hover);
}
.list-item.active {
  background: var(--bg-active);
  color: var(--ink-primary);
  font-weight: 500;
}
.list-item.linked:not(.active) {
  box-shadow: inset 2px 0 0 var(--border-soft);
}
.archived-row {
  opacity: 0.65;
}
.item-main {
  min-width: 0;
  flex: 1;
}
.item-top-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.state-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.state-dot.unread {
  background: #2563eb;
}
.state-spinner {
  color: #3b82f6;
  flex-shrink: 0;
  animation: sbSpin 0.85s linear infinite;
}
@keyframes sbSpin {
  to {
    transform: rotate(360deg);
  }
}
.item-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.rename-input {
  width: 100%;
  padding: 1px 4px;
  border: 1px solid var(--accent);
  border-radius: 4px;
  background: var(--bg-canvas);
  font-size: 12.5px;
  outline: none;
}
.item-right {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-shrink: 0;
}
.doc-mini-icon {
  color: var(--ink-faint);
}
.item-time {
  font-size: 11px;
  color: var(--ink-faint);
}
.more-dots {
  display: none;
  padding: 1px 5px;
  border-radius: 5px;
  font-size: 12px;
  color: var(--ink-muted);
  background: var(--bg-active);
}
.list-item:hover .more-dots {
  display: inline-flex;
}
.list-item:hover .item-time,
.list-item:hover .doc-mini-icon {
  display: none;
}
.session-menu {
  position: fixed;
  z-index: 70;
  width: 196px;
  background: var(--bg-canvas);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  padding: 6px;
  box-shadow: 0 12px 32px rgba(26, 25, 24, 0.14), 0 2px 6px rgba(26, 25, 24, 0.06);
}
.smenu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13.5px;
  color: var(--ink-primary);
}
.smenu-item:hover {
  background: var(--bg-subtle);
}
.smenu-item.danger {
  color: #b91c1c;
}
.kbd {
  font-size: 12px;
  color: var(--ink-faint);
}
.smenu-sep {
  height: 1px;
  background: var(--border-hairline);
  margin: 5px 8px;
}
.doc-list-item {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 6px 6px 10px;
  border-radius: 7px;
  font-size: 13px;
  color: var(--ink-body);
  cursor: pointer;
  margin-bottom: 1px;
}
.doc-more-dots {
  display: none;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 20px;
  padding: 0;
  border-radius: 5px;
  font-size: 12px;
  color: var(--ink-muted);
  background: var(--bg-active);
  flex-shrink: 0;
}
.doc-list-item:hover .doc-more-dots {
  display: inline-flex;
}
.doc-more-dots:hover {
  color: var(--ink-primary);
}
.doc-list-item:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.doc-list-item.active {
  background: var(--bg-active);
  color: var(--ink-primary);
  font-weight: 500;
}
.doc-list-icon {
  flex-shrink: 0;
  color: var(--ink-muted);
}
.doc-list-item.active .doc-list-icon {
  color: var(--ink-primary);
}
.toc-item {
  display: block;
  width: 100%;
  text-align: left;
  font-size: 12.5px;
  color: var(--ink-muted);
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.toc-item:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.empty-muted {
  font-size: 12px;
  color: var(--ink-faint);
  padding: 12px 10px;
}
.sidebar-footer {
  height: 46px;
  padding: 0 10px 0 12px;
  border-top: 1px solid var(--border-hairline);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.ws-select {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink-body);
  padding: 4px 2px;
  outline: none;
  cursor: pointer;
}
</style>
