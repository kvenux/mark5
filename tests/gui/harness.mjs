import { execFileSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const PNG_1X1 = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  'base64'
)

export const DEFAULT_TECH_DOC =
  `---
title: "深入理解 Rust 异步调度"
date: 2026-11-25
tags: [rust, tokio]
---

# 深入理解 Rust 异步调度

在高并发服务场景下，异步调度器的工作窃取机制决定了尾延迟上限。本文通过真实压测对比异步调度与异步队列的性能表现。

## 1. \`tokio::spawn\` 任务队列

\`\`\`rust
pub async fn handle(req: Request) -> Response {
    process(req).await
}
\`\`\`

## 2. 压测数据对比

| 方案 | QPS | P99 延迟 |
| --- | --- | --- |
| 同步线程池 | 18,400 | 42.1ms |
| 异步调度器 | 96,200 | 3.8ms |

![本地架构图](./arch.png)

` +
  Array.from(
    { length: 12 },
    (_, i) => `补充压测记录第 ${i + 1} 段：用于验证真实二进制下的滚动记忆与大纲跳转。`
  ).join('\n\n') +
  '\n'

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

export function seedWorkspace(wsDir) {
  fs.rmSync(wsDir, { recursive: true, force: true })
  fs.mkdirSync(path.join(wsDir, 'posts'), { recursive: true })
  fs.mkdirSync(path.join(wsDir, 'src'), { recursive: true })
  fs.writeFileSync(path.join(wsDir, 'posts', 'arch.png'), PNG_1X1)
  fs.writeFileSync(path.join(wsDir, 'posts', 'rust-async.md'), DEFAULT_TECH_DOC, 'utf8')
  fs.writeFileSync(
    path.join(wsDir, 'notes.md'),
    '# 随手记\n\n第二篇参考文档内容，用于验证多标签切换、副本创建与撤销栈独立保存。\n',
    'utf8'
  )
  fs.writeFileSync(
    path.join(wsDir, 'src', 'config.ts'),
    'export interface Mark5Config {\n  theme: "warm" | "light" | "dark";\n  editorMaxWidth: number;\n}\n',
    'utf8'
  )
}

export async function runRealBinarySuite({
  suiteId,
  suiteName,
  videoFile,
  exePath,
  outDir,
  cdpPort,
  fn
}) {
  const startedAt = Date.now()
  const tmpRoot = path.join(outDir, '_tmp', `${suiteId}-${startedAt}`)
  const wsDir = path.join(tmpRoot, 'ws')
  const wvDir = path.join(tmpRoot, 'wv2')
  const frameDir = path.join(tmpRoot, 'frames')
  const videoPath = path.join(outDir, videoFile)

  fs.mkdirSync(wvDir, { recursive: true })
  fs.mkdirSync(frameDir, { recursive: true })
  seedWorkspace(wsDir)

  const mark5 = spawn(exePath, [], {
    env: {
      ...process.env,
      MARK5_WORKSPACE: wsDir,
      WEBVIEW2_USER_DATA_FOLDER: wvDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`
    },
    stdio: 'ignore'
  })

  const errors = []
  const assertions = []
  let nextDialogHandler = null
  let ws = null
  let recording = false
  let frameIdx = 0
  let captureLoop = null
  let runError = null

  try {
    let wsUrl = ''
    for (let i = 0; i < 60; i++) {
      try {
        const res = await fetch(`http://127.0.0.1:${cdpPort}/json/list`)
        const list = await res.json()
        const page = list.find((t) => t.type === 'page' && t.webSocketDebuggerUrl)
        if (page) {
          wsUrl = page.webSocketDebuggerUrl
          break
        }
      } catch {}
      await sleep(250)
    }
    if (!wsUrl) throw new Error(`CDP connect failed on port ${cdpPort}`)

    ws = new WebSocket(wsUrl)
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', resolve, { once: true })
      ws.addEventListener('error', reject, { once: true })
    })

    let cid = 0
    const pending = new Map()
    const send = (method, params = {}) =>
      new Promise((resolve, reject) => {
        const id = ++cid
        pending.set(id, { resolve, reject })
        ws.send(JSON.stringify({ id, method, params }))
      })

    ws.addEventListener('message', (ev) => {
      const data = JSON.parse(String(ev.data))
      if (data.id && pending.has(data.id)) {
        const { resolve, reject } = pending.get(data.id)
        pending.delete(data.id)
        if (data.error) reject(new Error(data.error.message))
        else resolve(data.result)
      } else if (data.method === 'Runtime.exceptionThrown') {
        errors.push(
          'EXCEPTION: ' + JSON.stringify(data.params.exceptionDetails?.text || data.params)
        )
      } else if (data.method === 'Runtime.consoleAPICalled' && data.params.type === 'error') {
        errors.push(
          'CONSOLE_ERROR: ' +
            data.params.args.map((a) => a.value || a.description || '').join(' ')
        )
      } else if (data.method === 'Page.javascriptDialogOpening') {
        const handler = nextDialogHandler
        nextDialogHandler = null
        const accept = handler?.accept ?? true
        const promptText = handler?.promptText
        send('Page.handleJavaScriptDialog', promptText !== undefined ? { accept, promptText } : { accept }).catch(
          () => {}
        )
      }
    })

    const evalJs = async (expression) => {
      const r = await send('Runtime.evaluate', {
        expression: `(async () => { ${expression} })()`,
        returnByValue: true,
        awaitPromise: true
      })
      if (r.exceptionDetails) {
        throw new Error(
          'evalJs: ' +
            (r.exceptionDetails.exception?.description || r.exceptionDetails.text || 'unknown')
        )
      }
      return r.result?.value
    }

    const pressKey = async (k, code, vk, modifiers = 0) => {
      await send('Input.dispatchKeyEvent', {
        type: 'rawKeyDown',
        key: k,
        code,
        windowsVirtualKeyCode: vk,
        modifiers
      })
      await send('Input.dispatchKeyEvent', {
        type: 'keyUp',
        key: k,
        code,
        windowsVirtualKeyCode: vk,
        modifiers
      })
    }

    const insertText = async (text) => {
      await send('Input.insertText', { text })
    }

    const waitFor = async (expr, timeoutMs = 5000, label = expr) => {
      const deadline = Date.now() + timeoutMs
      while (Date.now() < deadline) {
        const val = await evalJs(`return Boolean(${expr});`)
        if (val) return true
        await sleep(120)
      }
      throw new Error(`waitFor timeout (${timeoutMs}ms): ${label}`)
    }

    const assert = (cond, msg) => {
      assertions.push({ ok: Boolean(cond), msg })
      if (!cond) throw new Error(`Assertion failed: ${msg}`)
    }

    const expectDialog = (accept = true, promptText = undefined) => {
      nextDialogHandler = { accept, promptText }
    }

    const mouseDrag = async (x1, y1, x2, y2, steps = 8) => {
      await send('Input.dispatchMouseEvent', {
        type: 'mouseMoved',
        x: x1,
        y: y1
      })
      await send('Input.dispatchMouseEvent', {
        type: 'mousePressed',
        x: x1,
        y: y1,
        button: 'left',
        clickCount: 1
      })
      for (let i = 1; i <= steps; i++) {
        const x = Math.round(x1 + ((x2 - x1) * i) / steps)
        const y = Math.round(y1 + ((y2 - y1) * i) / steps)
        await send('Input.dispatchMouseEvent', {
          type: 'mouseMoved',
          x,
          y,
          buttons: 1
        })
        await sleep(30)
      }
      await send('Input.dispatchMouseEvent', {
        type: 'mouseReleased',
        x: x2,
        y: y2,
        button: 'left',
        clickCount: 1
      })
    }

    const selectWordInFirstParagraph = async (word) => {
      const found = await evalJs(`
        const p = document.querySelector('.mu-container p');
        if (!p) return false;
        p.click();
        const w = document.createTreeWalker(p, NodeFilter.SHOW_TEXT);
        let tn;
        while ((tn = w.nextNode())) {
          const i = tn.textContent.indexOf(${JSON.stringify(word)});
          if (i >= 0) {
            const r = document.createRange();
            r.setStart(tn, i);
            r.setEnd(tn, i + ${word.length});
            const s = window.getSelection();
            s.removeAllRanges();
            s.addRange(r);
            p.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
            return true;
          }
        }
        return false;
      `)
      assert(found, `Selected word "${word}" in first paragraph`)
      await sleep(320)
    }

    await send('Page.enable')
    await send('Runtime.enable')
    await waitFor("document.querySelectorAll('.mu-container > *').length > 0", 12000, 'Muya mounted')
    await sleep(400)
    await evalJs(
      `Array.from(document.querySelectorAll('[title]')).find(n => (n.getAttribute('title')||'').includes('rust-async.md'))?.click();`
    )
    await sleep(350)

    // Start mandatory 10 FPS continuous video capture
    recording = true
    captureLoop = (async () => {
      while (recording) {
        const t0 = Date.now()
        try {
          const { data } = await send('Page.captureScreenshot', { format: 'jpeg', quality: 84 })
          fs.writeFileSync(
            path.join(frameDir, `frame_${String(frameIdx++).padStart(4, '0')}.jpg`),
            Buffer.from(data, 'base64')
          )
        } catch {}
        const dt = Date.now() - t0
        if (dt < 100) await sleep(100 - dt)
      }
    })()

    const ctx = {
      wsDir,
      send,
      evalJs,
      pressKey,
      insertText,
      waitFor,
      assert,
      expectDialog,
      mouseDrag,
      selectWordInFirstParagraph,
      sleep
    }

    await fn(ctx)
  } catch (err) {
    runError = err instanceof Error ? err.message : String(err)
  } finally {
    recording = false
    if (captureLoop) await captureLoop.catch(() => {})
    if (ws) {
      try {
        ws.close()
      } catch {}
    }
    try {
      mark5.kill()
    } catch {}
    await sleep(250)

    if (frameIdx > 0) {
      try {
        execFileSync(
          'ffmpeg',
          [
            '-y',
            '-framerate',
            '10',
            '-i',
            path.join(frameDir, 'frame_%04d.jpg'),
            '-vf',
            'scale=trunc(iw/2)*2:trunc(ih/2)*2',
            '-c:v',
            'libx264',
            '-pix_fmt',
            'yuv420p',
            '-crf',
            '22',
            videoPath
          ],
          { stdio: 'ignore' }
        )
      } catch (e) {
        runError = (runError ? runError + ' | ' : '') + 'ffmpeg failed: ' + e.message
      }
    }

    try {
      fs.rmSync(tmpRoot, { recursive: true, force: true })
    } catch {}
  }

  const videoBytes = fs.existsSync(videoPath) ? fs.statSync(videoPath).size : 0
  const ok = !runError && errors.length === 0 && videoBytes > 0

  return {
    id: suiteId,
    name: suiteName,
    ok,
    durationMs: Date.now() - startedAt,
    frames: frameIdx,
    videoPath,
    videoBytes,
    assertionsPassed: assertions.filter((a) => a.ok).length,
    assertionsTotal: assertions.length,
    errors,
    runError
  }
}
