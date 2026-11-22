import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  AgentDocOp,
  AgentStreamEvent,
  CliStatusItem,
  CommentThread,
  FileEntry,
  HistorySnapshot,
  Mark5API,
  ModelOption,
  SessionRecord,
  WorkspaceInfo
} from '@shared/types'

export function installTauriMark5Bridge(): void {
  if (typeof window === 'undefined' || window.mark5) return

  ;(window as unknown as { __MARK5_RESOLVE_LOCAL_SRC__?: (p: string) => string }).__MARK5_RESOLVE_LOCAL_SRC__ =
    (localPath: string) => convertFileSrc(localPath)

  const api: Mark5API = {
    getDefaultWorkspace: () => invoke<WorkspaceInfo | null>('get_default_workspace'),
    pickWorkspaceFolder: () => invoke<WorkspaceInfo | null>('pick_workspace_folder'),
    listWorkspaceFiles: (rootPath) => invoke<FileEntry[]>('list_workspace_files', { rootPath }),
    listWorkspaceAllFiles: (rootPath) =>
      invoke<FileEntry[]>('list_workspace_all_files', { rootPath }),
    readMarkdownFile: (rootPath, relPath) =>
      invoke<string>('read_markdown_file', { rootPath, relPath }),
    writeMarkdownFile: (rootPath, relPath, content) =>
      invoke<void>('write_markdown_file', { rootPath, relPath, content }),
    renameEntry: (rootPath, oldRelPath, newName) =>
      invoke<string>('rename_entry', { rootPath, oldRelPath, newName }),
    duplicateFile: (rootPath, relPath) =>
      invoke<string>('duplicate_file', { rootPath, relPath }),
    deleteEntry: (rootPath, relPath) =>
      invoke<void>('delete_entry', { rootPath, relPath }),
    createFolder: (rootPath, parentRelPath, folderName) =>
      invoke<string>('create_folder', { rootPath, parentRelPath, folderName }),
    revealInExplorer: (rootPath, relPath) =>
      invoke<void>('reveal_in_explorer', { rootPath, relPath }),

    listSessions: (rootPath) => invoke<SessionRecord[]>('list_sessions', { rootPath }),
    createSession: (payload) => invoke<SessionRecord>('create_session', { payload }),
    saveSession: (rootPath, session) => invoke<void>('save_session', { rootPath, session }),
    deleteSession: (rootPath, sessionId) =>
      invoke<void>('delete_session', { rootPath, sessionId }),

    listThreads: (rootPath, docRelPath) =>
      invoke<CommentThread[]>('list_threads', { rootPath, docRelPath }),
    saveThreads: (rootPath, docRelPath, threads) =>
      invoke<void>('save_threads', { rootPath, docRelPath, threads }),

    listHistory: (rootPath, docRelPath) =>
      invoke<HistorySnapshot[]>('list_history', { rootPath, docRelPath }),
    saveHistorySnapshot: (rootPath, payload) =>
      invoke<HistorySnapshot>('save_history_snapshot', { rootPath, payload }),

    sendAgentTurn: (payload) => invoke<void>('send_agent_turn', { payload }),
    abortAgentTurn: (sessionId) => invoke<void>('abort_agent_turn', { sessionId }),

    syncLiveDocument: (payload) => {
      void invoke<void>('sync_live_document', {
        rootPath: payload.rootPath,
        docRelPath: payload.docRelPath,
        markdown: payload.markdown
      })
    },

    exportDocument: async (payload) => {
      if (payload.format === 'pdf') {
        window.print()
        return { ok: true, outPath: '系统打印 / 另存为 PDF' }
      }
      return invoke<{ ok: boolean; outPath?: string }>('export_document', { payload })
    },

    openNewWindow: () => invoke<void>('open_new_window'),
    checkCliStatus: (paths) =>
      invoke<CliStatusItem[]>('check_cli_status', {
        agyPath: paths?.agyPath || null,
        claudePath: paths?.claudePath || null,
        codexPath: paths?.codexPath || null
      }),
    listEngineModels: (engine, cliPath) =>
      invoke<ModelOption[]>('list_engine_models', {
        engine,
        cliPath: cliPath || null
      }),

    onAgentEvent: (cb) => {
      let unlisten: (() => void) | null = null
      let disposed = false
      void listen<AgentStreamEvent>('mc:agent-event', (ev) => {
        if (!disposed) cb(ev.payload)
      }).then((fn) => {
        if (disposed) fn()
        else unlisten = fn
      })
      return () => {
        disposed = true
        unlisten?.()
      }
    },

    onAgentDocOp: (cb) => {
      let unlisten: (() => void) | null = null
      let disposed = false
      void listen<AgentDocOp>('mc:agent-doc-op', (ev) => {
        if (!disposed) cb(ev.payload)
      }).then((fn) => {
        if (disposed) fn()
        else unlisten = fn
      })
      return () => {
        disposed = true
        unlisten?.()
      }
    }
  }

  window.mark5 = api
}
