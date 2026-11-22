import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

const PRUNED_FONT_RE =
  /(DejaVuSansMono|open-sans-|KaTeX_|NotoSansMath|NotoMusic|danielbd)/i

function pruneHeavyAssetsPlugin(): Plugin {
  const STUB_VEGA = '\0stub:vega-embed'
  const STUB_KATEX = '\0stub:katex'
  const STUB_EMPTY_CSS = '\0stub:empty-css'

  return {
    name: 'mark5-prune-heavy-assets',
    enforce: 'pre',
    resolveId(source) {
      if (source === 'vega-embed' || source === 'vega-lite' || source === 'vega') {
        return STUB_VEGA
      }
      if (source === 'katex' || source === 'katex/contrib/mhchem') {
        return STUB_KATEX
      }
      if (/katex.*\.css$/i.test(source)) {
        return STUB_EMPTY_CSS
      }
      return null
    },
    load(id) {
      if (id === STUB_VEGA) {
        return `export default async function vegaEmbed() { return { view: null }; }\nexport const vega = {};\n`
      }
      if (id === STUB_KATEX) {
        return `export function render(expr, el) { if (el) el.textContent = String(expr || ''); }\nexport function renderToString(expr) { return String(expr || ''); }\nexport default { render, renderToString };\n`
      }
      if (id === STUB_EMPTY_CSS) {
        return ''
      }
      return null
    },
    transform(code, id) {
      if (!id.endsWith('.css')) return null
      if (!PRUNED_FONT_RE.test(code)) return null
      // Strip @font-face blocks that reference pruned offline fonts so Vite never emits them
      const cleaned = code.replace(/@font-face\s*\{[\s\S]*?\}/g, (block) =>
        PRUNED_FONT_RE.test(block) ? '' : block
      )
      return { code: cleaned, map: null }
    },
    generateBundle(_options, bundle) {
      for (const fileName of Object.keys(bundle)) {
        if (/\.(ttf|woff|woff2)$/i.test(fileName) && PRUNED_FONT_RE.test(fileName)) {
          delete bundle[fileName]
        }
      }
    }
  }
}

export default defineConfig({
  root: path.resolve(__dirname, 'src/renderer'),
  base: './',
  plugins: [pruneHeavyAssetsPlugin(), vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src/renderer/src'),
      '@shared': path.resolve(__dirname, 'src/shared')
    }
  },
  server: {
    port: 1420,
    strictPort: true
  },
  build: {
    outDir: path.resolve(__dirname, 'dist'),
    emptyOutDir: true
  }
})
