<template>
  <div class="tree-node">
    <div
      v-if="entry.isDir"
      class="tree-row dir-row"
      :style="{ paddingLeft: `${depth * 14 + 8}px` }"
      @click="toggle"
    >
      <ChevronDown v-if="isOpen" :size="13" :stroke-width="1.75" class="tree-chev" />
      <ChevronRight v-else :size="13" :stroke-width="1.75" class="tree-chev" />
      <FolderOpen v-if="isOpen" :size="14" :stroke-width="1.75" class="tree-icon" />
      <Folder v-else :size="14" :stroke-width="1.75" class="tree-icon" />
      <span class="tree-name">{{ entry.name }}</span>
    </div>

    <div
      v-else
      class="tree-row file-row"
      :class="{ active: entry.relPath === activePath }"
      :style="{ paddingLeft: `${depth * 14 + 22}px` }"
      @click="$emit('select', entry.relPath)"
    >
      <FileText v-if="fileKind === 'md'" :size="14" :stroke-width="1.75" class="tree-icon" />
      <FileCode v-else-if="fileKind === 'code'" :size="14" :stroke-width="1.75" class="tree-icon" />
      <File v-else :size="14" :stroke-width="1.75" class="tree-icon" />
      <span class="tree-name">{{ entry.name }}</span>
    </div>

    <template v-if="entry.isDir && isOpen && entry.children?.length">
      <FileTreeNode
        v-for="child in entry.children"
        :key="child.relPath"
        :entry="child"
        :depth="depth + 1"
        :active-path="activePath"
        :force-open="forceOpen"
        @select="$emit('select', $event)"
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
}>()

defineEmits<{
  (e: 'select', relPath: string): void
}>()

const expanded = ref(props.depth < 1)
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
  padding: 5px 8px;
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
</style>
