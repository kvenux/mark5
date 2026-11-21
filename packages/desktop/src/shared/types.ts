export type AgentEngine = 'agy' | 'claude' | 'codex'
export type AppTheme = 'warm' | 'light' | 'dark'

export interface AppSettings {
  theme: AppTheme
  editorFontSize: number
  editorMaxWidth: number
  defaultEngine: AgentEngine
  models: Record<AgentEngine, string>
  cliPaths: Record<AgentEngine, string>
}

export interface CliStatusItem {
  engine: AgentEngine
  available: boolean
  version: string
  resolvedPath: string
}

export interface WorkspaceInfo {
  id: string
  name: string
  rootPath: string
}

export interface FileEntry {
  name: string
  relPath: string
  isDir: boolean
  children?: FileEntry[]
}

export interface ToolTraceItem {
  id: string
  name: string
  state: 'running' | 'done' | 'error'
  detail?: string
  durationSec?: number
}

export interface QuotedSnippet {
  text: string
  docRelPath?: string
}

export interface ChatMessage {
  id: string
  role: 'user' | 'agent' | 'status' | 'error'
  text: string
  timestamp: number
  streaming?: boolean
  tools?: ToolTraceItem[]
  quotes?: QuotedSnippet[]
  touchedDocs?: string[]
}

export interface SessionRecord {
  id: string
  workspacePath: string
  title: string
  engine: AgentEngine
  model?: string
  conversationId?: string
  docRelPath?: string
  isArtifact?: boolean
  pinned?: boolean
  archived?: boolean
  unread?: boolean
  createdAt: number
  updatedAt: number
  messages: ChatMessage[]
}

export interface CommentMessage {
  id: string
  who: 'user' | 'agent'
  text: string
  timestamp: number
}

export interface CommentThread {
  id: string
  docRelPath: string
  blockIndex: number
  quote: string
  resolved: boolean
  createdBy: 'user' | 'agent'
  messages: CommentMessage[]
  createdAt: number
}

export interface HistorySnapshot {
  id: string
  docRelPath: string
  timestamp: number
  actor: 'user' | 'agent'
  summary: string
  markdown: string
}

export interface SkeletonSectionInput {
  heading: string
  plan: string
}

export type AgentDocOp =
  | {
      type: 'create_skeleton'
      sessionId: string
      docRelPath: string
      title: string
      author: string
      sections: SkeletonSectionInput[]
    }
  | {
      type: 'replace_block'
      sessionId: string
      docRelPath: string
      blockIndex: number
      newMarkdown: string
      label: string
      resultingMarkdown?: string
    }
  | {
      type: 'insert_block'
      sessionId: string
      docRelPath: string
      afterBlockIndex: number
      newMarkdown: string
      label: string
      resultingMarkdown?: string
    }
  | {
      type: 'delete_block'
      sessionId: string
      docRelPath: string
      blockIndex: number
      label: string
      resultingMarkdown?: string
    }
  | {
      type: 'reply_comment'
      sessionId: string
      docRelPath: string
      threadId: string
      replyText: string
    }
  | {
      type: 'create_comment'
      sessionId: string
      docRelPath: string
      blockIndex: number
      quote: string
      questionText: string
    }
  | {
      type: 'tab_note'
      sessionId: string
      docRelPath: string
      noteText: string
    }

export interface AgentStreamEvent {
  sessionId: string
  kind: 'init' | 'tool' | 'delta' | 'status' | 'done' | 'error'
  conversationId?: string
  stepIndex?: number
  toolName?: string
  toolState?: 'running' | 'done' | 'error'
  toolDetail?: string
  durationSec?: number
  text?: string
  error?: string
}

export interface Mark5API {
  // Workspace & files
  getDefaultWorkspace(): Promise<WorkspaceInfo>
  pickWorkspaceFolder(): Promise<WorkspaceInfo | null>
  listWorkspaceFiles(rootPath: string): Promise<FileEntry[]>
  listWorkspaceAllFiles(rootPath: string): Promise<FileEntry[]>
  readMarkdownFile(rootPath: string, relPath: string): Promise<string>
  writeMarkdownFile(rootPath: string, relPath: string, content: string): Promise<void>
  renameEntry?: (rootPath: string, oldRelPath: string, newName: string) => Promise<string>
  duplicateFile?: (rootPath: string, relPath: string) => Promise<string>
  deleteEntry?: (rootPath: string, relPath: string) => Promise<void>
  createFolder?: (rootPath: string, parentRelPath: string, folderName: string) => Promise<string>
  revealInExplorer?: (rootPath: string, relPath: string) => Promise<void>

  // Sessions & metadata (.mark5/)
  listSessions(rootPath: string): Promise<SessionRecord[]>
  createSession(payload: {
    rootPath: string
    title: string
    engine: AgentEngine
    initialPrompt?: string
  }): Promise<SessionRecord>
  saveSession(rootPath: string, session: SessionRecord): Promise<void>
  deleteSession(rootPath: string, sessionId: string): Promise<void>

  // Comment threads
  listThreads(rootPath: string, docRelPath: string): Promise<CommentThread[]>
  saveThreads(rootPath: string, docRelPath: string, threads: CommentThread[]): Promise<void>

  // History snapshots
  listHistory(rootPath: string, docRelPath: string): Promise<HistorySnapshot[]>
  saveHistorySnapshot(
    rootPath: string,
    payload: Omit<HistorySnapshot, 'id' | 'timestamp'>
  ): Promise<HistorySnapshot>

  // Headless Agent (AGY Phase 1)
  sendAgentTurn(payload: {
    sessionId: string
    rootPath: string
    prompt: string
    model?: string
    cliPath?: string
    docRelPath?: string
    currentMarkdown?: string
    threadContext?: {
      threadId: string
      blockIndex: number
      quote: string
      history: CommentMessage[]
    }
  }): Promise<void>
  abortAgentTurn(sessionId: string): Promise<void>

  // Sync latest live markdown from renderer so MCP doc_read_blocks always sees human edits
  syncLiveDocument(payload: { rootPath: string; docRelPath: string; markdown: string }): void

  // Export (Markdown / HTML / PDF)
  exportDocument(payload: {
    rootPath: string
    docRelPath: string
    format: 'html' | 'pdf' | 'md'
    markdown: string
    html?: string
  }): Promise<{ ok: boolean; outPath?: string; message?: string }>

  // Multi-window & CLI diagnostics
  openNewWindow?: () => Promise<void>
  checkCliStatus?: (paths?: {
    agyPath?: string
    claudePath?: string
    codexPath?: string
  }) => Promise<CliStatusItem[]>

  // Subscriptions from main -> renderer
  onAgentEvent(cb: (ev: AgentStreamEvent) => void): () => void
  onAgentDocOp(cb: (op: AgentDocOp) => void): () => void
}
