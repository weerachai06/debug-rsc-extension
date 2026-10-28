import { resolve } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';
import pkg from './package.json' with { type: 'json' };

const manifest = {
  manifest_version: 3,
  name: 'RSC Debugger',
  version: pkg.version,
  devtools_page: 'devtools.html',
  content_security_policy: {
    extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self';",
  },
  content_scripts: [
    { matches: ['http://*/*', 'https://*/*'], js: ['content/hook.js'], run_at: 'document_start', world: 'MAIN' },
    { matches: ['http://*/*', 'https://*/*'], js: ['content/relay.js'], run_at: 'document_start' },
  ],
};

const emitManifest = (): Plugin => ({
  name: 'emit-manifest',
  generateBundle() {
    this.emitFile({ type: 'asset', fileName: 'manifest.json', source: JSON.stringify(manifest, null, 2) });
  },
});

// The panel and DevTools pages. Content scripts are built separately by vite.content.config.ts,
// because they must be self-contained classic scripts.
export default defineConfig({
  root: 'src',
  base: './',
  plugins: [react(), emitManifest()],
  build: {
    outDir: '../.output/chrome-mv3',
    emptyOutDir: true,
    rollupOptions: { input: { panel: resolve('src/panel.html'), devtools: resolve('src/devtools.html') } },
  },
});
