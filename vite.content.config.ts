import { resolve } from 'node:path';
import { defineConfig } from 'vite';

// One self-contained IIFE per content script. Run with `--mode hook` and `--mode relay`.
export default defineConfig(({ mode }) => ({
  root: 'src',
  publicDir: false,
  build: {
    outDir: '../.output/chrome-mv3',
    emptyOutDir: false,
    lib: { entry: resolve(`src/content/${mode}.ts`), formats: ['iife'], name: mode, fileName: () => `content/${mode}.js` },
  },
}));
