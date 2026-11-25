import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { runRealBinarySuite } from './harness.mjs'
import { GUI_SUITES } from './specs/suites.mjs'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.resolve(__dirname, '..', '..')
const DEFAULT_EXE = path.join(
  REPO_ROOT,
  'packages',
  'desktop',
  'src-tauri',
  'target',
  'release',
  'mark5.exe'
)

function parseArgs(argv) {
  const opts = { suites: null, build: false, outDir: null, exePath: DEFAULT_EXE }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--build') opts.build = true
    else if (a === '--suite' && argv[i + 1]) {
      opts.suites = argv[++i]
        .split(',')
        .map((s) => s.trim().toUpperCase())
        .filter(Boolean)
    } else if (a === '--out' && argv[i + 1]) {
      opts.outDir = path.resolve(argv[++i])
    } else if (a === '--exe' && argv[i + 1]) {
      opts.exePath = path.resolve(argv[++i])
    }
  }
  return opts
}

function toFileUri(p) {
  return 'file:///' + path.resolve(p).replace(/\\/g, '/')
}

async function main() {
  const opts = parseArgs(process.argv.slice(2))

  if (opts.build) {
    console.log('[gui-test] Building release mark5.exe via pnpm build ...')
    execFileSync(process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm', ['build'], {
      cwd: REPO_ROOT,
      stdio: 'inherit'
    })
  }

  if (!fs.existsSync(opts.exePath)) {
    console.error(`[gui-test] mark5.exe not found at: ${opts.exePath}`)
    console.error('[gui-test] Run `pnpm test:gui:build` or `pnpm build` first.')
    process.exit(1)
  }

  const runId = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const outDir = opts.outDir || path.join(REPO_ROOT, 'tests', 'gui', 'artifacts', runId)
  fs.mkdirSync(outDir, { recursive: true })

  const selected = opts.suites
    ? GUI_SUITES.filter((s) => opts.suites.includes(s.id))
    : GUI_SUITES

  if (!selected.length) {
    console.error(`[gui-test] No matching suites for filter: ${opts.suites?.join(',')}`)
    process.exit(1)
  }

  console.log(`[gui-test] Running ${selected.length} real-binary GUI suites -> ${outDir}`)
  const results = []

  for (let idx = 0; idx < selected.length; idx++) {
    const suite = selected[idx]
    const cdpPort = 9310 + idx
    console.log(`[gui-test] (${idx + 1}/${selected.length}) ${suite.id}: ${suite.name} ...`)
    const res = await runRealBinarySuite({
      suiteId: suite.id,
      suiteName: suite.name,
      videoFile: suite.videoFile,
      exePath: opts.exePath,
      outDir,
      cdpPort,
      fn: suite.run
    })
    results.push(res)
    const status = res.ok ? 'PASS' : 'FAIL'
    console.log(
      `[gui-test]   -> ${status} | ${res.assertionsPassed}/${res.assertionsTotal} assertions | ${res.frames} frames | ${(res.videoBytes / 1024).toFixed(0)} KB | errors=${res.errors.length}${res.runError ? ' | ' + res.runError : ''}`
    )
  }

  const reportJson = {
    runId,
    exePath: opts.exePath,
    outDir,
    passed: results.filter((r) => r.ok).length,
    total: results.length,
    results
  }
  fs.writeFileSync(path.join(outDir, 'report.json'), JSON.stringify(reportJson, null, 2), 'utf8')

  const mdLines = [
    `# Mark 5 Real-Binary GUI Test Report (${runId})`,
    '',
    `- **Binary**: \`${opts.exePath}\``,
    `- **Result**: **${reportJson.passed} / ${reportJson.total} PASSED**`,
    '',
    '| Suite | Name | Status | Assertions | Frames | Errors | Video (Mandatory Visual Review) |',
    '|---|---|---|---:|---:|---:|---|',
    ...results.map(
      (r) =>
        `| ${r.id} | ${r.name} | ${r.ok ? 'PASS' : 'FAIL'} | ${r.assertionsPassed}/${r.assertionsTotal} | ${r.frames} | ${r.errors.length} | [${path.basename(r.videoPath)}](${toFileUri(r.videoPath)}) |`
    ),
    ''
  ]
  const failures = results.filter((r) => !r.ok)
  if (failures.length) {
    mdLines.push('## Failure Details', '')
    for (const f of failures) {
      mdLines.push(`### ${f.id} — ${f.name}`)
      if (f.runError) mdLines.push(`- Error: \`${f.runError}\``)
      for (const e of f.errors) mdLines.push(`- Console/Runtime: \`${e}\``)
      mdLines.push('')
    }
  }
  fs.writeFileSync(path.join(outDir, 'report.md'), mdLines.join('\n'), 'utf8')

  console.log(`\n[gui-test] Report: ${path.join(outDir, 'report.md')}`)
  console.log(`[gui-test] Summary: ${reportJson.passed}/${reportJson.total} PASSED`)
  process.exit(failures.length === 0 ? 0 : 1)
}

main().catch((err) => {
  console.error('[gui-test] Fatal:', err)
  process.exit(1)
})
