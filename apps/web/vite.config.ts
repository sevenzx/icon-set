import { sveltekit } from '@sveltejs/kit/vite';
import { execFileSync } from 'node:child_process';
import { relative, resolve } from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';

const rootDir = fileURLToPath(new URL('../..', import.meta.url));
const docsSyncScript = resolve(rootDir, 'scripts/sync-docs.mjs');
const docsToWatch = [resolve(rootDir, 'docs/github-repo-config.md')];

function syncDocs() {
  execFileSync(process.execPath, [docsSyncScript], { stdio: 'inherit' });
}

function docsSyncPlugin(): Plugin {
  let serverConfigured = false;

  return {
    name: 'icon-set-docs-sync',
    buildStart() {
      if (serverConfigured) return;
      syncDocs();
    },
    configureServer(server) {
      serverConfigured = true;
      syncDocs();
      server.watcher.add(docsToWatch);
      server.watcher.on('change', (changedPath) => {
        if (!docsToWatch.includes(changedPath)) return;
        syncDocs();
        server.config.logger.info(
          `[sync-docs] updated ${relative(rootDir, changedPath)}`
        );
      });
    }
  };
}

export default defineConfig({
  plugins: [docsSyncPlugin(), sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: process.env.VITE_API_PROXY_TARGET ?? 'http://127.0.0.1:3000',
        changeOrigin: true
      }
    }
  }
});
