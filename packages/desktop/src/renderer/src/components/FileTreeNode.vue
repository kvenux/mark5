<template>
  <div class="tree-node">
    <div
      v-if="entry.isDir"
      class="tree-row dir-row"
      :style="{ paddingLeft: `${depth * 14 + 8}px` }"
      @click="toggle"
      @contextmenu.prevent.stop="$emit('open-menu', entry, $event)"
    >
      <ChevronDown v-if="isOpen" :size="13" :stroke-width="1.75" class="tree-chev" />
      <ChevronRight v-else :size="13" :stroke-width="1.75" class="tree-chev" />
      <FolderOpen v-if="isOpen" :size="14" :stroke-width="1.75" class="tree-icon" />
      <Folder v-else :size="14" :stroke-width="1.75" class="tree-icon" />
      <input
        v-if="renamingPath === entry.relPath"
        class="tree-rename-input"
        :value="renameDraft"
        @click.stop
        @input="$emit('update:rename-draft', ($event.target as HTMLInputElement).value)"
        @keydown.enter.stop="$emit('commit-rename')"
        @keydown.esc.stop="$emit('cancel-rename')"
        @blur="$emit('commit-rename')"
      />
      <span v-else class="tree-name">{{ entry.name }}</span>
      <button
        v-if="renamingPath !== entry.relPath"
        class="tree-more"
        title="更多操作"
        @click.stop="$emit('open-menu', entry, $event)"
      >
        ···
      </button>
    </div>

    <div
      v-else
      class="tree-row file-row"
      :class="{ active: entry.relPath === activePath }"
      :style="{ paddingLeft: `${depth * 14 + 22}px` }"
      @click="$emit('select', entry.relPath)"
      @contextmenu.prevent.stop="$emit('open-menu', entry, $event)"
    >
      <FileText v-if="fileKind === 'md'" :size="14" :stroke-width="1.75" class="tree-icon" />
      <FileCode v-else-if="fileKind === 'code'" :size="14" :stroke-width="1.75" class="tree-icon" />
      <File v-else :size="14" :stroke-width="1.75" class="tree-icon" />
      <input
        v-if="renamingPath === entry.relPath"
        class="tree-rename-input"
        :value="renameDraft"
        @click.stop
        @input="$emit('update:rename-draft', ($event.target as HTMLInputElement).value)"
        @keydown.enter.stop="$emit('commit-rename')"
        @keydown.esc.stop="$emit('cancel-rename')"
        @blur="$emit('commit-rename')"
      />
      <span v-else class="tree-name">{{ entry.name }}</span>
      <button
        v-if="renamingPath !== entry.relPath"
        class="tree-more"
        title="更多操作"
        @click.stop="$emit('open-menu', entry, $event)"
      >
        ···
      </button>
    </div>

    <template v-if="entry.isDir && isOpen && entry.children?.length">
      <FileTreeNode
        v-for="child in entry.children"
        :key="child.relPath"
        :entry="child"
        :depth="depth + 1"
        :active-path="activePath"
        :force-open="forceOpen"
        :renaming-path="renamingPath"
        :rename-draft="renameDraft"
        @select="$emit('select', $event)"
        @open-menu="(ent, ev) => $emit('open-menu', ent, ev)"
        @update:rename-draft="$emit('update:rename-draft', $event)"
        @commit-rename="$emit('commit-rename')"
        @cancel-rename="$emit('cancel-rename')"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, ChevronRight, File, FileCode, FileText, Folder, FolderOpen } from 'lucide-vue-next'
import type { FileEntry } from '@shared/types'

defineOptions({ name: 'FileTreeNode' })

const props = defineProps<{
  entry: FileEntry
  depth: number
  activePath: string | null
  forceOpen?: boolean
  renamingPath?: string | null
  renameDraft?: string
}>()

defineEmits<{
  (e: 'select', relPath: string): void
  (e: 'open-menu', entry: FileEntry, ev: MouseEvent): void
  (e: 'update:rename-draft', val: string): void
  (e: 'commit-rename'): void
  (e: 'cancel-rename'): void
}>()

const expanded = ref(false)
const isOpen = computed(() => Boolean(props.forceOpen || expanded.value))
const fileKind = computed<'md' | 'code' | 'other'>(() => {
  const n = props.entry.name.toLowerCase()
  if (/\.(md|markdown)$/.test(n)) return 'md'
  if (/\.(ts|tsx|js|jsx|mjs|cjs|vue|json|css|scss|html|py|go|rs|java|c|cpp|h|sh|yml|yaml|toml|sql)$/.test(n)) {
    return 'code'
  }
  return 'other'
})

function toggle() {
  expanded.value = !expanded.value
}
</script>

<style scoped>
.tree-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 6px 5px 8px;
  border-radius: 7px;
  font-size: 13px;
  color: var(--ink-body);
  cursor: pointer;
  user-select: none;
  margin-bottom: 1px;
}
.tree-row:hover {
  background: var(--bg-hover);
  color: var(--ink-primary);
}
.tree-row.active {
  background: var(--bg-active);
  color: var(--ink-primary);
  font-weight: 500;
}
.tree-chev {
  flex-shrink: 0;
  color: var(--ink-faint);
}
.tree-icon {
  flex-shrink: 0;
  color: var(--ink-muted);
}
.tree-row.active .tree-icon {
  color: var(--ink-primary);
}
.tree-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.tree-more {
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
.tree-row:hover .tree-more {
  display: inline-flex;
}
.tree-more:hover {
  color: var(--ink-primary);
}
.tree-rename-input {
  flex: 1;
  min-width: 0;
  padding: 1px 6px;
  border: 1px solid var(--accent);
  border-radius: 5px;
  font-size: 12.5px;
  background: var(--bg-canvas);
  color: var(--ink-primary);
  outline: none;
}
</style>
