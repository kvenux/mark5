<template>
  <main class="welcome-root">
    <div class="welcome-card">
      <header class="welcome-hero">
        <div class="welcome-logo">Mark 5</div>
        <p class="welcome-sub">本地 Markdown 协同写作空间</p>
      </header>

      <section class="welcome-actions">
        <button class="action-card primary" @click="store.pickWorkspace()">
          <FolderOpen :size="18" :stroke-width="1.75" />
          <div class="action-meta">
            <span class="action-title">打开项目目录</span>
            <span class="action-desc">选择本地文件夹开始写作或协同</span>
          </div>
        </button>

        <button class="action-card" @click="$emit('open-settings')">
          <Settings :size="18" :stroke-width="1.75" />
          <div class="action-meta">
            <span class="action-title">偏好与引擎设置</span>
            <span class="action-desc">切换主题、配置 AGY / Claude / Codex</span>
          </div>
        </button>
      </section>

      <section class="welcome-recent">
        <div class="recent-head">
          <span>最近打开</span>
          <span v-if="store.workspaces.length" class="recent-count">
            {{ store.workspaces.length }}
          </span>
        </div>

        <div v-if="store.workspaces.length" class="recent-list">
          <div
            v-for="ws in store.workspaces.slice(0, 6)"
            :key="ws.rootPath"
            class="recent-row"
            @click="openRecent(ws)"
          >
            <Folder :size="15" :stroke-width="1.75" class="recent-ico" />
            <div class="recent-info">
              <span class="recent-name">{{ ws.name }}</span>
              <span class="recent-path" :title="ws.rootPath">{{ shortPath(ws.rootPath) }}</span>
            </div>
            <button
              class="recent-remove"
              title="从最近列表移除"
              @click.stop="store.removeRecentWorkspace(ws.rootPath)"
            >
              <X :size="13" :stroke-width="1.9" />
            </button>
          </div>
        </div>
        <div v-else class="recent-empty">
          还没有最近打开的目录，点击上方「打开项目目录」即可开始。
        </div>
      </section>

      <footer class="welcome-foot">
        <span class="foot-hint"><kbd>Ctrl+Shift+N</kbd> 新建窗口</span>
        <span class="foot-sep">·</span>
        <span class="foot-hint"><kbd>Ctrl+,</kbd> 打开设置</span>
      </footer>
    </div>
  </main>
</template>

<script setup lang="ts">
import { Folder, FolderOpen, Settings, X } from 'lucide-vue-next'
import type { WorkspaceInfo } from '@shared/types'
import { useAppStore } from '@/stores/appStore'

defineEmits<{
  (e: 'open-settings'): void
}>()

const store = useAppStore()

async function openRecent(ws: WorkspaceInfo) {
  await store.switchWorkspace(ws)
}

function shortPath(p: string): string {
  const norm = p.replace(/\\/g, '/')
  const homeMatch = norm.match(/^([A-Za-z]:\/Users\/[^/]+)(\/.*)$/)
  if (homeMatch) return `~${homeMatch[2]}`
  return p
}
</script>

<style scoped>
.welcome-root {
  flex: 1;
  width: 100%;
  height: 100%;
  background: var(--bg-canvas);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px 24px;
  overflow-y: auto;
}
.welcome-card {
  width: 100%;
  max-width: 520px;
  display: flex;
  flex-direction: column;
  gap: 26px;
}
.welcome-hero {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.welcome-logo {
  font-size: 26px;
  font-weight: 650;
  letter-spacing: -0.03em;
  color: var(--ink-primary);
}
.welcome-sub {
  margin: 0;
  font-size: 13.5px;
  color: var(--ink-muted);
}
.welcome-actions {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}
.action-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 15px;
  border-radius: 12px;
  border: 1px solid var(--border-soft);
  background: var(--bg-sidebar);
  color: var(--ink-primary);
  text-align: left;
  transition: border-color 0.14s ease, background 0.14s ease;
}
.action-card:hover {
  background: var(--bg-subtle);
  border-color: var(--ink-faint);
}
.action-card.primary {
  border-color: var(--border-soft);
  background: var(--bg-subtle);
}
.action-card.primary svg {
  color: var(--accent);
}
.action-meta {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.action-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--ink-primary);
}
.action-desc {
  font-size: 11.5px;
  line-height: 1.45;
  color: var(--ink-muted);
}
.welcome-recent {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.recent-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  color: var(--ink-muted);
  padding: 0 2px;
}
.recent-count {
  font-size: 11px;
  color: var(--ink-faint);
}
.recent-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  border: 1px solid var(--border-hairline);
  border-radius: 12px;
  padding: 4px;
  background: var(--bg-sidebar);
}
.recent-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s ease;
}
.recent-row:hover {
  background: var(--bg-hover);
}
.recent-ico {
  color: var(--ink-muted);
  flex-shrink: 0;
}
.recent-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  flex: 1;
}
.recent-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--ink-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.recent-path {
  font-size: 11.5px;
  color: var(--ink-muted);
  font-family: Consolas, 'JetBrains Mono', monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.recent-remove {
  width: 24px;
  height: 24px;
  padding: 0;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-faint);
  opacity: 0;
  flex-shrink: 0;
}
.recent-row:hover .recent-remove {
  opacity: 1;
}
.recent-remove:hover {
  background: var(--bg-active);
  color: var(--ink-primary);
}
.recent-empty {
  padding: 18px 14px;
  border: 1px dashed var(--border-soft);
  border-radius: 12px;
  font-size: 12.5px;
  color: var(--ink-muted);
  text-align: center;
}
.welcome-foot {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  font-size: 11.5px;
  color: var(--ink-faint);
}
.foot-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.foot-hint kbd {
  font-family: Consolas, 'JetBrains Mono', monospace;
  font-size: 10.5px;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--border-hairline);
  background: var(--bg-subtle);
  color: var(--ink-muted);
}
.foot-sep {
  opacity: 0.5;
}
</style>
