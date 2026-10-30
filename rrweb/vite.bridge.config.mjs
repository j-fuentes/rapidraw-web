import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const r = (p) => fileURLToPath(new URL(p, import.meta.url));
export default defineConfig({
  root: r('./bridge'),
  envPrefix: ['VITE_'],
  define: {
    __RR_VERSION__: JSON.stringify(JSON.parse(readFileSync(r('../src-tauri/tauri.conf.json'), 'utf8')).version),
    __RR_WEB_VERSION__: JSON.stringify(readFileSync(r('./VERSION'), 'utf8').trim()), // fork: rrweb/VERSION
  },
  build: { outDir: r('./dist-bridge'), emptyOutDir: true },
});
