import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type {
  AgentDocOp,
  AgentEngine,
  AgentStreamEvent,
  CommentThread,
  FileEntry,
  HistorySnapshot,
  SessionRecord,
  WorkspaceInfo
} from '@shared/types'

export interface QueuedBlockAnim {
  id: string
  op: AgentDocOp
}

export const useAppStore = defineStore('app', () => {
  const workspaces = ref<WorkspaceInfo[]>([])
  const activeWorkspace = ref<WorkspaceInfo | null>(null)
  const files = ref<FileEntry[]>([])
  const allFiles = ref<FileEntry[]>([])
  const sessions = ref<SessionRecord[]>([])
  const activeSessionId = ref<string | null>(null)
  const savedEng = (localStorage.getItem('mark5_engine') || 'agy') as AgentEngine
  const preferredEngine = ref<AgentEngine>(
    savedEng === 'claude' || savedEng === 'codex' ? savedEng : 'agy'
  )

  const openTabs = ref<string[]>([])
  const activeDocRelPath = ref<string | null>(null)
  const markdown = ref<string>('')
  const externalDocSeq = ref(0)
  const toc = ref<Array<{ lvl: number; content: string; slug: string }>>([])
  const scrollHeadingRequest = ref<{ text: string; index: number; seq: number }>({
    text: '',
    index: -1,
    seq: 0
  })
  const quotedSnippets = ref<Array<{ id: string; text: string; docRelPath?: string }>>([])
  const chatDraftRequest = ref<{ text: string; seq: number }>({ text: '', seq: 0 })
  const threads = ref<CommentThread[]>([])
  const history = ref<HistorySnapshot[]>([])

  const sessionBusyMap = ref<Record<string, boolean>>({})
  const sessionLiveStatusMap = ref<Record<string, string>>({})
  const sessionTurnSeqMap = new Map<string, number>()
  const pendingDocOps = ref<QueuedBlockAnim[]>([])
  const toastMessage = ref('')
  let toastTimer: ReturnType<typeof setTimeout> | null = null
  const activeTurnMsgId = new Map<string, string>()

  const activeSession = computed(
    () => sessions.value.find((s) => s.id === activeSessionId.value) ?? null
  )
  const agentBusy = computed({
    get: () => Boolean(activeSessionId.value && sessionBusyMap.value[activeSessionId.value]),
    set: (val: boolean) => {
      if (activeSessionId.value) sessionBusyMap.value[activeSessionId.value] = val
    }
  })
  const agentPhase = computed({
    get: () => (activeSessionId.value ? sessionLiveStatusMap.value[activeSessionId.value] || '' : ''),
    set: (val: string) => {
      if (activeSessionId.value) sessionLiveStatusMap.value[activeSessionId.value] = val
    }
  })

  function showToast(msg: string) {
    toastMessage.value = msg
    if (toastTimer) clearTimeout(toastTimer)
    toastTimer = setTimeout(() => {
      toastMessage.value = ''
    }, 2600)
  }

  async function init() {
    const ws = await window.mark5.getDefaultWorkspace()
    workspaces.value = [ws]
    await switchWorkspace(ws)

    window.mark5.onAgentEvent(handleAgentEvent)
    window.mark5.onAgentDocOp(handleAgentDocOp)
  }

  async function pickWorkspace() {
    const ws = await window.mark5.pickWorkspaceFolder()
    if (!ws) return
    if (!workspaces.value.some((w) => w.rootPath === ws.rootPath)) {
      workspaces.value.push(ws)
    }
    await switchWorkspace(ws)
  }

  function clearEditorState() {
    openTabs.value = []
    pendingDocOps.value = []
    toc.value = []
    activeDocRelPath.value = null
    markdown.value = ''
    threads.value = []
    history.value = []
    externalDocSeq.value += 1
  }

  function requestScrollToHeading(text: string, index = -1) {
    scrollHeadingRequest.value = { text, index, seq: scrollHeadingRequest.value.seq + 1 }
  }

  function quoteToChat(quote: string) {
    const clean = quote.trim()
    if (!clean) return
    if (!quotedSnippets.value.some((q) => q.text === clean)) {
      quotedSnippets.value.push({
        id: `q-${Date.now()}-${Math.random().toString(36).slice(2, 5)}`,
        text: clean,
        docRelPath: activeDocRelPath.value ?? undefined
      })
    }
    chatDraftRequest.value = {
      text: clean,
      seq: chatDraftRequest.value.seq + 1
    }
  }

  function removeQuote(id: string) {
    quotedSnippets.value = quotedSnippets.value.filter((q) => q.id !== id)
  }

  function clearQuotes() {
    quotedSnippets.value = []
  }

  function findFirstFileRelPath(list: FileEntry[]): string | null {
    for (const item of list) {
      if (!item.isDir) return item.relPath
      if (item.children?.length) {
        const nested = findFirstFileRelPath(item.children)
        if (nested) return nested
      }
    }
    return null
  }

  async function switchWorkspace(ws: WorkspaceInfo) {
    clearEditorState()
    activeWorkspace.value = ws
    const [mdTree, fullTree, sessList] = await Promise.all([
      window.mark5.listWorkspaceFiles(ws.rootPath),
      window.mark5.listWorkspaceAllFiles(ws.rootPath),
      window.mark5.listSessions(ws.rootPath)
    ])
    files.value = mdTree
    allFiles.value = fullTree
    sessions.value = sessList

    const firstActive = sessions.value.find((s) => !s.archived)
    if (firstActive) {
      await selectSession(firstActive.id)
    } else {
      activeSessionId.value = null
    }
    if (!activeDocRelPath.value) {
      const firstMd = findFirstFileRelPath(files.value)
      if (firstMd) await openDocument(firstMd)
    }
  }

  async function refreshFiles() {
    if (!activeWorkspace.value) return
    const [mdTree, fullTree] = await Promise.all([
      window.mark5.listWorkspaceFiles(activeWorkspace.value.rootPath),
      window.mark5.listWorkspaceAllFiles(activeWorkspace.value.rootPath)
    ])
    files.value = mdTree
    allFiles.value = fullTree
  }

  async function setActiveSessionEngine(engine: AgentEngine) {
    preferredEngine.value = engine
    localStorage.setItem('mark5_engine', engine)
    const sess = activeSession.value
    if (sess && sess.messages.length === 0 && activeWorkspace.value) {
      sess.engine = engine
      await window.mark5.saveSession(activeWorkspace.value.rootPath, JSON.parse(JSON.stringify(sess)))
    }
  }

  async function createSession(title: string, engine: AgentEngine, initialPrompt?: string) {
    if (!activeWorkspace.value) return null
    const session = await window.mark5.createSession({
      rootPath: activeWorkspace.value.rootPath,
      title,
      engine
    })
    sessions.value.unshift(session)
    activeSessionId.value = session.id

    if (initialPrompt && initialPrompt.trim()) {
      await sendChatPrompt(initialPrompt.trim())
    }
    return session
  }

  async function selectSession(sessionId: string) {
    activeSessionId.value = sessionId
    const sess = sessions.value.find((s) => s.id === sessionId)
    if (sess && sess.unread) {
      sess.unread = false
      if (activeWorkspace.value) {
        await window.mark5.saveSession(
          activeWorkspace.value.rootPath,
          JSON.parse(JSON.stringify(sess))
        )
      }
    }
    if (sess?.docRelPath) {
      await openDocument(sess.docRelPath)
    }
  }

  async function togglePinSession(sessionId: string) {
    const sess = sessions.value.find((s) => s.id === sessionId)
    if (!sess || !activeWorkspace.value) return
    sess.pinned = !sess.pinned
    if (sess.pinned) sess.archived = false
    await window.mark5.saveSession(activeWorkspace.value.rootPath, JSON.parse(JSON.stringify(sess)))
  }

  async function renameSession(sessionId: string, newTitle: string) {
    const trimmed = newTitle.trim()
    const sess = sessions.value.find((s) => s.id === sessionId)
    if (!sess || !trimmed || !activeWorkspace.value) return
    sess.title = trimmed
    await window.mark5.saveSession(activeWorkspace.value.rootPath, JSON.parse(JSON.stringify(sess)))
  }

  async function toggleArchiveSession(sessionId: string) {
    const sess = sessions.value.find((s) => s.id === sessionId)
    if (!sess || !activeWorkspace.value) return
    sess.archived = !sess.archived
    if (sess.archived) sess.pinned = false
    await window.mark5.saveSession(activeWorkspace.value.rootPath, JSON.parse(JSON.stringify(sess)))
    if (sess.archived && activeSessionId.value === sessionId) {
      const next = sessions.value.find((s) => !s.archived)
      if (next) await selectSession(next.id)
      else {
        activeSessionId.value = null
        clearEditorState()
      }
    }
  }

  async function deleteSession(sessionId: string) {
    if (!activeWorkspace.value) return
    await stopSession(sessionId)
    await window.mark5.deleteSession(activeWorkspace.value.rootPath, sessionId)
    sessions.value = sessions.value.filter((s) => s.id !== sessionId)
    if (activeSessionId.value === sessionId) {
      const next = sessions.value.find((s) => !s.archived)
      if (next) await selectSession(next.id)
      else {
        activeSessionId.value = null
        clearEditorState()
      }
    }
  }

  async function stopSession(sessionId: string) {
    await window.mark5.abortAgentTurn(sessionId)
    sessionBusyMap.value[sessionId] = false
    sessionLiveStatusMap.value[sessionId] = ''
  }

  async function openDocument(relPath: string) {
    if (!activeWorkspace.value) return
    const [content, docThreads, docHistory] = await Promise.all([
      window.mark5.readMarkdownFile(activeWorkspace.value.rootPath, relPath),
      window.mark5.listThreads(activeWorkspace.value.rootPath, relPath),
      window.mark5.listHistory(activeWorkspace.value.rootPath, relPath)
    ])
    if (!openTabs.value.includes(relPath)) {
      openTabs.value.push(relPath)
    }
    markdown.value = content
    threads.value = docThreads
    history.value = docHistory
    activeDocRelPath.value = relPath
    externalDocSeq.value += 1
  }

  async function closeTab(relPath: string) {
    const idx = openTabs.value.indexOf(relPath)
    if (idx === -1) return
    openTabs.value.splice(idx, 1)
    if (activeDocRelPath.value === relPath) {
      const next = openTabs.value[Math.min(idx, openTabs.value.length - 1)]
      if (next) {
        await openDocument(next)
      } else {
        clearEditorState()
      }
    }
  }

  async function onEditorMarkdownChanged(newMd: string, saveToDisk = true) {
    markdown.value = newMd
    if (!activeWorkspace.value || !activeDocRelPath.value) return
    window.mark5.syncLiveDocument({
      rootPath: activeWorkspace.value.rootPath,
      docRelPath: activeDocRelPath.value,
      markdown: newMd
    })
    if (saveToDisk) {
      await window.mark5.writeMarkdownFile(
        activeWorkspace.value.rootPath,
        activeDocRelPath.value,
        newMd
      )
    }
  }

  async function appendToCurrentDocument(snippet: string) {
    if (!activeWorkspace.value || !activeDocRelPath.value) return
    if (!/\.(md|markdown)$/i.test(activeDocRelPath.value)) {
      showToast('当前打开的不是 Markdown 文档')
      return
    }
    const clean = snippet.trim()
    if (!clean) return
    const nextMd = markdown.value.trimEnd() + '\n\n' + clean + '\n'
    markdown.value = nextMd
    externalDocSeq.value += 1
    await onEditorMarkdownChanged(nextMd, true)
    showToast(`已插入到 ${activeDocRelPath.value}`)
  }

  async function sendChatPrompt(
    prompt: string,
    quotes?: Array<{ text: string; docRelPath?: string }>
  ) {
    if (!activeWorkspace.value) return
    const cleanPrompt = prompt.trim()
    const cleanQuotes = (quotes || []).filter((q) => q.text.trim())
    if (!cleanPrompt) return

    const autoTitle =
      cleanPrompt
        .replace(/^(帮我写个|帮我写|写个|请帮我|请)/, '')
        .replace(/\s+/g, ' ')
        .trim()
        .slice(0, 24) || '新写作会话'

    let sess = activeSession.value
    if (!sess) {
      sess = await createSession(autoTitle, preferredEngine.value)
      if (!sess) return
    } else if (
      (sess.title === '新会话' || sess.title === '新写作会话') &&
      !sess.messages.some((m) => m.role === 'user')
    ) {
      sess.title = autoTitle
    }

    sess.messages.push({
      id: `m-${Date.now()}`,
      role: 'user',
      text: cleanPrompt,
      quotes: cleanQuotes.length ? cleanQuotes : undefined,
      timestamp: Date.now()
    })
    sess.updatedAt = Date.now()
    await window.mark5.saveSession(activeWorkspace.value.rootPath, JSON.parse(JSON.stringify(sess)))

    const agentPrompt = cleanQuotes.length
      ? `${cleanQuotes
          .map((q) => (q.docRelPath ? `> [来自 ${q.docRelPath}] ${q.text}` : `> ${q.text}`))
          .join('\n\n')}\n\n${cleanPrompt}`
      : cleanPrompt

    const mySeq = (sessionTurnSeqMap.get(sess.id) ?? 0) + 1
    sessionTurnSeqMap.set(sess.id, mySeq)
    sessionBusyMap.value[sess.id] = true
    sessionLiveStatusMap.value[sess.id] = '正在启动 AGY...'
    try {
      await window.mark5.sendAgentTurn({
        sessionId: sess.id,
        rootPath: activeWorkspace.value.rootPath,
        prompt: agentPrompt,
        docRelPath: activeDocRelPath.value ?? sess.docRelPath,
        currentMarkdown: markdown.value
      })
    } catch (err) {
      if (sessionTurnSeqMap.get(sess.id) === mySeq) {
        sessionLiveStatusMap.value[sess.id] = ''
        sess.messages.push({
          id: `err-${Date.now()}`,
          role: 'error',
          text: err instanceof Error ? err.message : String(err),
          timestamp: Date.now()
        })
      }
    } finally {
      if (sessionTurnSeqMap.get(sess.id) === mySeq) {
        sessionBusyMap.value[sess.id] = false
      }
    }
  }

  async function sendSelectionComment(blockIndex: number, quote: string, text: string) {
    if (!activeWorkspace.value || !activeDocRelPath.value) return
    let sess = activeSession.value
    if (!sess) {
      sess = await createSession(activeDocRelPath.value.replace(/\.md$/i, ''), preferredEngine.value)
      if (!sess) return
    }

    const rand = Math.random().toString(36).slice(2, 6)
    const thread: CommentThread = {
      id: `thr-${Date.now().toString(36)}-${rand}`,
      docRelPath: activeDocRelPath.value,
      blockIndex,
      quote,
      resolved: false,
      createdBy: 'user',
      createdAt: Date.now(),
      messages: [
        {
          id: `cm-${Date.now()}-${rand}`,
          who: 'user',
          text,
          timestamp: Date.now()
        }
      ]
    }
    threads.value.unshift(thread)
    await persistThreads()

    const targetSessionId = sess.id
    const mySeq = (sessionTurnSeqMap.get(targetSessionId) ?? 0) + 1
    sessionTurnSeqMap.set(targetSessionId, mySeq)
    sessionBusyMap.value[targetSessionId] = true
    sessionLiveStatusMap.value[targetSessionId] = '正在处理批注...'
    try {
      await window.mark5.sendAgentTurn({
        sessionId: targetSessionId,
        rootPath: activeWorkspace.value.rootPath,
        prompt: text,
        docRelPath: activeDocRelPath.value,
        currentMarkdown: markdown.value,
        threadContext: {
          threadId: thread.id,
          blockIndex,
          quote,
          history: JSON.parse(JSON.stringify(thread.messages))
        }
      })
    } catch (err) {
      if (sessionTurnSeqMap.get(targetSessionId) === mySeq) {
        sessionLiveStatusMap.value[targetSessionId] = ''
        showToast(`批注处理失败：${err instanceof Error ? err.message : String(err)}`)
      }
    } finally {
      if (sessionTurnSeqMap.get(targetSessionId) === mySeq) {
        sessionBusyMap.value[targetSessionId] = false
      }
    }
  }

  async function replyInThread(threadId: string, text: string) {
    if (!activeWorkspace.value || !activeDocRelPath.value || !activeSession.value) return
    const thread = threads.value.find((t) => t.id === threadId)
    if (!thread) return
    thread.messages.push({
      id: `cm-${Date.now()}`,
      who: 'user',
      text,
      timestamp: Date.now()
    })
    await persistThreads()

    const targetSessionId = activeSession.value.id
    const mySeq = (sessionTurnSeqMap.get(targetSessionId) ?? 0) + 1
    sessionTurnSeqMap.set(targetSessionId, mySeq)
    sessionBusyMap.value[targetSessionId] = true
    sessionLiveStatusMap.value[targetSessionId] = '正在回复批注...'
    try {
      await window.mark5.sendAgentTurn({
        sessionId: targetSessionId,
        rootPath: activeWorkspace.value.rootPath,
        prompt: text,
        docRelPath: activeDocRelPath.value,
        currentMarkdown: markdown.value,
        threadContext: {
          threadId: thread.id,
          blockIndex: thread.blockIndex,
          quote: thread.quote,
          history: JSON.parse(JSON.stringify(thread.messages))
        }
      })
    } catch (err) {
      if (sessionTurnSeqMap.get(targetSessionId) === mySeq) {
        sessionLiveStatusMap.value[targetSessionId] = ''
        showToast(`批注回复失败：${err instanceof Error ? err.message : String(err)}`)
      }
    } finally {
      if (sessionTurnSeqMap.get(targetSessionId) === mySeq) {
        sessionBusyMap.value[targetSessionId] = false
      }
    }
  }

  async function resolveThread(threadId: string) {
    const t = threads.value.find((x) => x.id === threadId)
    if (!t) return
    t.resolved = true
    await persistThreads()
  }

  async function persistThreads() {
    if (!activeWorkspace.value || !activeDocRelPath.value) return
    await window.mark5.saveThreads(
      activeWorkspace.value.rootPath,
      activeDocRelPath.value,
      JSON.parse(JSON.stringify(threads.value))
    )
  }

  async function rollbackToSnapshot(snap: HistorySnapshot) {
    if (!activeWorkspace.value || !activeDocRelPath.value) return
    if (markdown.value && markdown.value !== snap.markdown) {
      const backup = await window.mark5.saveHistorySnapshot(activeWorkspace.value.rootPath, {
        docRelPath: activeDocRelPath.value,
        actor: 'user',
        summary: '回滚前自动备份',
        markdown: markdown.value
      })
      history.value.unshift(backup)
    }
    await onEditorMarkdownChanged(snap.markdown, true)
    externalDocSeq.value += 1
    showToast(`已回滚至快照 (${snap.summary})`)
  }

  function ensureStreamingMsg(sess: SessionRecord) {
    const existingId = activeTurnMsgId.get(sess.id)
    if (existingId) {
      const found = sess.messages.find((m) => m.id === existingId)
      if (found) return found
    }
    const msg = {
      id: `a-${Date.now()}-${Math.random().toString(36).slice(2, 5)}`,
      role: 'agent' as const,
      text: '',
      timestamp: Date.now(),
      streaming: true,
      tools: []
    }
    sess.messages.push(msg)
    activeTurnMsgId.set(sess.id, msg.id)
    return msg
  }

  function handleAgentEvent(ev: AgentStreamEvent) {
    const sess = sessions.value.find((s) => s.id === ev.sessionId)
    if (!sess) return
    if (ev.conversationId) {
      sess.conversationId = ev.conversationId
    }

    if (ev.kind === 'status') {
      sessionLiveStatusMap.value[sess.id] = ev.text || '执行中...'
      ensureStreamingMsg(sess)
      return
    }

    if (ev.kind === 'init') {
      sessionLiveStatusMap.value[sess.id] = '已连接，思考中...'
      ensureStreamingMsg(sess)
      return
    }

    if (ev.kind === 'tool') {
      const msg = ensureStreamingMsg(sess)
      const tools = msg.tools ?? (msg.tools = [])
      const toolId = `step-${ev.stepIndex ?? tools.length}-${ev.toolName ?? 'tool'}`
      const existing = tools.find((t) => t.id === toolId)
      const state = ev.toolState || 'running'
      if (existing) {
        existing.state = state
        if (ev.durationSec !== undefined) existing.durationSec = ev.durationSec
        if (ev.toolDetail) existing.detail = ev.toolDetail
      } else {
        tools.push({
          id: toolId,
          name: ev.toolName || 'tool',
          state,
          detail: ev.toolDetail,
          durationSec: ev.durationSec
        })
      }
      const readable = ev.toolDetail || ev.toolName || '工具'
      sessionLiveStatusMap.value[sess.id] =
        state === 'running' ? `正在${readable}...` : `已完成${readable}`
      return
    }

    if (ev.kind === 'delta' && ev.text) {
      const msg = ensureStreamingMsg(sess)
      msg.text += ev.text
      sessionLiveStatusMap.value[sess.id] = '正在生成回复...'
      return
    }

    if (ev.kind === 'done') {
      sessionBusyMap.value[sess.id] = false
      sessionLiveStatusMap.value[sess.id] = ''
      const msg = ensureStreamingMsg(sess)
      msg.streaming = false
      if (ev.text && ev.text.trim()) {
        msg.text = ev.text.trim()
      } else if (!msg.text.trim()) {
        msg.text = msg.tools && msg.tools.length > 0 ? '已完成。' : '已完成处理。'
      } else {
        msg.text = msg.text.trim()
      }
      activeTurnMsgId.delete(sess.id)
      sess.updatedAt = Date.now()
      if (activeSessionId.value !== sess.id) {
        sess.unread = true
      }
      if (activeWorkspace.value) {
        window.mark5.saveSession(
          activeWorkspace.value.rootPath,
          JSON.parse(JSON.stringify(sess))
        )
      }
      return
    }

    if (ev.kind === 'error') {
      sessionBusyMap.value[sess.id] = false
      sessionLiveStatusMap.value[sess.id] = ''
      const existingId = activeTurnMsgId.get(sess.id)
      if (existingId) {
        const m = sess.messages.find((x) => x.id === existingId)
        if (m) m.streaming = false
      }
      activeTurnMsgId.delete(sess.id)
      if (activeSessionId.value !== sess.id) {
        sess.unread = true
      }
      sess.messages.push({
        id: `err-${Date.now()}`,
        role: 'error',
        text: ev.error || 'AGY 执行失败',
        timestamp: Date.now()
      })
      if (activeWorkspace.value) {
        window.mark5.saveSession(
          activeWorkspace.value.rootPath,
          JSON.parse(JSON.stringify(sess))
        )
      }
    }
  }

  function recordTouchedDocOnActiveTurn(sess: SessionRecord | undefined, docRelPath: string) {
    if (!sess || !docRelPath) return
    const turnId = activeTurnMsgId.get(sess.id)
    const msg = turnId
      ? sess.messages.find((m) => m.id === turnId)
      : [...sess.messages].reverse().find((m) => m.role === 'agent')
    if (!msg) return
    if (!msg.touchedDocs) msg.touchedDocs = []
    if (!msg.touchedDocs.includes(docRelPath)) {
      msg.touchedDocs.push(docRelPath)
    }
  }

  async function handleAgentDocOp(op: AgentDocOp) {
    const sess = sessions.value.find((s) => s.id === op.sessionId)

    if (op.type === 'create_skeleton') {
      recordTouchedDocOnActiveTurn(sess, op.docRelPath)
      if (sess && !sess.docRelPath) {
        sess.docRelPath = op.docRelPath
        sess.title = op.title
        if (activeWorkspace.value) {
          await window.mark5.saveSession(
            activeWorkspace.value.rootPath,
            JSON.parse(JSON.stringify(sess))
          )
        }
      }
      await refreshFiles()
      await openDocument(op.docRelPath)
      return
    }

    if (op.type === 'reply_comment') {
      if (op.docRelPath === activeDocRelPath.value) {
        const thr = threads.value.find((t) => t.id === op.threadId)
        if (thr) {
          thr.messages.push({
            id: `cm-a-${Date.now()}`,
            who: 'agent',
            text: op.replyText,
            timestamp: Date.now()
          })
          await persistThreads()
        }
      } else if (activeWorkspace.value) {
        const bgThreads = await window.mark5.listThreads(
          activeWorkspace.value.rootPath,
          op.docRelPath
        )
        const thr = bgThreads.find((t) => t.id === op.threadId)
        if (thr) {
          thr.messages.push({
            id: `cm-a-${Date.now()}`,
            who: 'agent',
            text: op.replyText,
            timestamp: Date.now()
          })
          await window.mark5.saveThreads(
            activeWorkspace.value.rootPath,
            op.docRelPath,
            bgThreads
          )
        }
      }
      return
    }

    if (op.type === 'create_comment') {
      const rand = Math.random().toString(36).slice(2, 6)
      const newThr: CommentThread = {
        id: `thr-a-${Date.now().toString(36)}-${rand}`,
        docRelPath: op.docRelPath,
        blockIndex: op.blockIndex,
        quote: op.quote,
        resolved: false,
        createdBy: 'agent',
        createdAt: Date.now(),
        messages: [
          {
            id: `cm-a-${Date.now()}-${rand}`,
            who: 'agent',
            text: op.questionText,
            timestamp: Date.now()
          }
        ]
      }
      if (op.docRelPath === activeDocRelPath.value) {
        threads.value.unshift(newThr)
        await persistThreads()
      } else if (activeWorkspace.value) {
        const bgThreads = await window.mark5.listThreads(
          activeWorkspace.value.rootPath,
          op.docRelPath
        )
        bgThreads.unshift(newThr)
        await window.mark5.saveThreads(
          activeWorkspace.value.rootPath,
          op.docRelPath,
          bgThreads
        )
      }
      return
    }

    // Queue block edits for paragraph-by-paragraph Agent Cursor animation
    if ('docRelPath' in op && op.docRelPath) {
      recordTouchedDocOnActiveTurn(sess, op.docRelPath)
    }
    pendingDocOps.value.push({
      id: `op-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      op
    })
  }

  function shiftPendingOp(): QueuedBlockAnim | undefined {
    return pendingDocOps.value.shift()
  }

  return {
    workspaces,
    activeWorkspace,
    files,
    allFiles,
    sessions,
    activeSessionId,
    activeSession,
    preferredEngine,
    setActiveSessionEngine,
    openTabs,
    activeDocRelPath,
    markdown,
    externalDocSeq,
    toc,
    scrollHeadingRequest,
    requestScrollToHeading,
    quotedSnippets,
    chatDraftRequest,
    quoteToChat,
    removeQuote,
    clearQuotes,
    threads,
    history,
    agentBusy,
    agentPhase,
    sessionBusyMap,
    sessionLiveStatusMap,
    pendingDocOps,
    toastMessage,
    showToast,
    init,
    pickWorkspace,
    switchWorkspace,
    refreshFiles,
    createSession,
    selectSession,
    togglePinSession,
    renameSession,
    toggleArchiveSession,
    deleteSession,
    stopSession,
    openDocument,
    closeTab,
    onEditorMarkdownChanged,
    appendToCurrentDocument,
    sendChatPrompt,
    sendSelectionComment,
    replyInThread,
    resolveThread,
    rollbackToSnapshot,
    shiftPendingOp
  }
})
