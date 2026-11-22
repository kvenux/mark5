import { spawnSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const EXE_PATH =
  process.env.MARK5_EXE ||
  path.join(
    REPO_ROOT,
    'packages',
    'desktop',
    'src-tauri',
    'target',
    'release',
    'mark5.exe'
  );
const BENCH_WS = path.join(__dirname, '.bench-ws');
const WV2_DIR = path.join(__dirname, '.bench-wv2');
const HISTORY_FILE = path.join(__dirname, '.mem-profile-history.json');

const label = process.argv[2] || 'baseline';
const waitSec = Number(process.argv[3] || 7);

function ensureStressWorkspace() {
  fs.mkdirSync(BENCH_WS, { recursive: true });
  const mermaidDoc =
    `# 复杂架构与多图表压力测试文档

本文档包含多个 Mermaid 流程图、时序图、状态图、大型表格与多语言代码块，用于测试 Mark 5 内存占用。

## 1. 系统整体架构图

\`\`\`mermaid
flowchart TD
    A[Mark 5 Tauri Shell] --> B[WebView2 Renderer]
    A --> C[Axum Streamable HTTP MCP]
    B --> D[Pinia AppStore]
    B --> E[Muya WYSIWYG Core]
    C --> F[AGY Headless CLI]
    C --> G[Claude Code CLI]
    C --> H[Codex CLI]
\`\`\`

## 2. 协同编辑时序图

\`\`\`mermaid
sequenceDiagram
    participant U as User
    participant M as Muya Editor
    participant S as Rust MCP Server
    participant A as Agent CLI
    U->>M: 划词发起批注
    M->>S: send_agent_turn
    S->>A: spawn --output-format stream-json
    A->>S: doc_replace_block
    S-->>M: emit mc:agent-doc-op
\`\`\`

## 3. 状态流转图

\`\`\`mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Locking: 收到 MCP 改块事件
    Locking --> Typing: 38% 黄金区居中
    Typing --> Settled: 单块实时提交 Muya
    Settled --> ReviewReady: 记入 PendingBlockChange
    ReviewReady --> Idle: 接受或退回
\`\`\`

` +
    Array.from(
      { length: 40 },
      (_, i) => `### 章节 ${i + 1}：高并发协同细节

在分布式本地协同场景下，第 **${i + 1}** 节重点分析 \`doc_replace_block\` 与 \`Muya\` 增量渲染之间的内存与延迟权衡。

| 指标项 | 单开基准 | 双开增量 | 三开增量 | 优化目标 |
| --- | ---: | ---: | ---: | ---: |
| Rust 后端 RSS | 6.2 MB | +4.1 MB | +4.0 MB | < 5 MB |
| WebView2 Renderer | 78.5 MB | +52.0 MB | +51.2 MB | < 45 MB |
| WebView2 GPU 共享 | 34.0 MB | +0.0 MB | +0.0 MB | 共享进程 |

\`\`\`rust
pub async fn handle_block_${i}(idx: usize, md: &str) -> Result<usize, String> {
    let blocks: Vec<&str> = md.split("\\n\\n").filter(|s| !s.trim().is_empty()).collect();
    Ok(blocks.get(idx).map(|s| s.len()).unwrap_or(0))
}
\`\`\`
`
    ).join('\n');

  fs.writeFileSync(path.join(BENCH_WS, '01-architecture-mermaid.md'), mermaidDoc, 'utf8');

  const massiveDoc =
    `# 超长技术规格说明书（约 70 KB）\n\n` +
    Array.from(
      { length: 160 },
      (_, i) => `## ${i + 1}. 模块设计规范与边界条件

> 核心约束：第 ${i + 1} 号模块必须保证在低内存环境下稳定运行，单实例 Private Working Set 可控。

- **输入校验**：严格检查 UTF-8 编码、路径穿越防护与原子落盘写入；
- **并发控制**：多窗口场景通过 \`mark5-doc-<pid>\` 隔离各自 MCP 端口；
- **性能指标**：P99 渲染帧耗时 < 16ms，大文档滚动无掉帧。
`
    ).join('\n');
  fs.writeFileSync(path.join(BENCH_WS, '02-massive-spec.md'), massiveDoc, 'utf8');
}

function sampleProcessTree(rootPids, wv2Tag) {
  const psScript = `
$roots = @(${rootPids.join(',')})
$all = Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('mark5.exe','msedgewebview2.exe') }
$perf = @{}
Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | ForEach-Object {
  $perf[[int]$_.IDProcess] = [math]::Round($_.WorkingSetPrivate / 1MB, 2)
}
$inc = [System.Collections.Generic.HashSet[int]]::new()
foreach ($r in $roots) { [void]$inc.Add([int]$r) }
$changed = $true
while ($changed) {
  $changed = $false
  foreach ($p in $all) {
    if ($inc.Contains([int]$p.ParentProcessId) -and -not $inc.Contains([int]$p.ProcessId)) {
      [void]$inc.Add([int]$p.ProcessId)
      $changed = $true
    }
  }
}
foreach ($p in $all) {
  if ($p.CommandLine -and ($p.CommandLine -like "*${wv2Tag}*")) {
    [void]$inc.Add([int]$p.ProcessId)
  }
}
$rows = @()
foreach ($p in $all) {
  if ($inc.Contains([int]$p.ProcessId)) {
    $priv = if ($perf.ContainsKey([int]$p.ProcessId)) { $perf[[int]$p.ProcessId] } else { 0.0 }
    $role = if ($p.Name -eq 'mark5.exe') { 'rust' }
      elseif ($p.CommandLine -match '--type=renderer') { 'renderer' }
      elseif ($p.CommandLine -match '--type=gpu-process') { 'gpu' }
      elseif ($p.CommandLine -match '--type=utility') {
        if ($p.CommandLine -match 'network') { 'network' }
        elseif ($p.CommandLine -match 'storage') { 'storage' }
        else { 'utility' }
      }
      elseif ($p.CommandLine -match '--type=crashpad') { 'crashpad' }
      else { 'browser' }
    $rows += [PSCustomObject]@{ pid=$p.ProcessId; role=$role; privMB=$priv }
  }
}
$rows | ConvertTo-Json -Compress
`;
  const res = spawnSync('pwsh', ['-NoProfile', '-Command', psScript], { encoding: 'utf8' });
  const raw = (res.stdout || '').trim();
  if (!raw) return { rows: [], byRole: {}, total: 0 };
  const parsed = JSON.parse(raw);
  const rows = Array.isArray(parsed) ? parsed : [parsed];
  const byRole = {};
  let total = 0;
  for (const r of rows) {
    byRole[r.role] = Number(((byRole[r.role] || 0) + r.privMB).toFixed(2));
    total += r.privMB;
  }
  return { rows, byRole, total: Number(total.toFixed(1)) };
}

function cleanupBench(pids, wv2Tag) {
  const pidList = pids.filter(Boolean).join(',');
  const ps = `
foreach ($id in @(${pidList})) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 600
Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*${wv2Tag}*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
`;
  spawnSync('pwsh', ['-NoProfile', '-Command', ps], { stdio: 'ignore' });
}

function minimizeProcessWindows(pid) {
  const ps = `
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinMin {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  public static void MinPid(uint target) {
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == target && IsWindowVisible(h)) ShowWindow(h, 6);
      return true;
    }, IntPtr.Zero);
  }
}
"@
[WinMin]::MinPid(${pid})
`;
  spawnSync('pwsh', ['-NoProfile', '-Command', ps], { stdio: 'ignore' });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function runScenario(name, windowCount, extraEnv = {}, settleSec = waitSec) {
  const tag = `bench-wv2-${Date.now()}`;
  const wv2Path = path.join(WV2_DIR, tag);
  fs.mkdirSync(wv2Path, { recursive: true });

  const env = {
    ...process.env,
    MARK5_WORKSPACE: BENCH_WS,
    WEBVIEW2_USER_DATA_FOLDER: wv2Path,
    MARK5_BENCH_WINDOWS: String(windowCount),
    ...extraEnv
  };

  const child = spawn(EXE_PATH, [], { env, stdio: 'ignore' });
  await sleep(settleSec * 1000);
  const active = sampleProcessTree([child.pid], tag);

  let afterIdle = null;
  if (extraEnv.MARK5_BENCH_IDLE_CHECK === '1') {
    minimizeProcessWindows(child.pid);
    await sleep(6000);
    afterIdle = sampleProcessTree([child.pid], tag);
  }

  cleanupBench([child.pid], tag);
  try {
    fs.rmSync(wv2Path, { recursive: true, force: true });
  } catch {}
  return { name, active, afterIdle };
}

ensureStressWorkspace();
if (!fs.existsSync(EXE_PATH)) {
  console.error('mark5.exe not found:', EXE_PATH);
  process.exit(1);
}

const exeSizeMB = Number((fs.statSync(EXE_PATH).size / 1024 / 1024).toFixed(2));
console.log(`\n[mem-profile] label="${label}" | exe=${exeSizeMB} MB`);

const s1 = await runScenario('S1-single', 1);
console.log('S1 (1 window, complex doc):', s1.active.total, 'MB', s1.active.byRole);

const s2 = await runScenario(
  'S2-dual-inapp',
  2,
  { MARK5_LOW_MEM_DELAY_SECS: '2', MARK5_BENCH_IDLE_CHECK: '1' },
  waitSec
);
console.log('S2 (2 in-app windows, active):', s2.active.total, 'MB', s2.active.byRole);
if (s2.afterIdle) {
  console.log('S2 (2 in-app windows, bg-low):', s2.afterIdle.total, 'MB', s2.afterIdle.byRole);
}

const record = {
  ts: new Date().toISOString(),
  label,
  exeSizeMB,
  s1: { total: s1.active.total, byRole: s1.active.byRole },
  s2Active: { total: s2.active.total, byRole: s2.active.byRole },
  s2BgLow: s2.afterIdle ? { total: s2.afterIdle.total, byRole: s2.afterIdle.byRole } : null,
  incremental2ndWin: Number((s2.active.total - s1.active.total).toFixed(1))
};

let hist = [];
if (fs.existsSync(HISTORY_FILE)) {
  try {
    hist = JSON.parse(fs.readFileSync(HISTORY_FILE, 'utf8'));
  } catch {}
}
hist.push(record);
fs.writeFileSync(HISTORY_FILE, JSON.stringify(hist, null, 2));

console.log('\n=== SUMMARY ===');
console.log(JSON.stringify(record, null, 2));
