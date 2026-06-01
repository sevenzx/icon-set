import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const docs = [
  {
    source: 'docs/github-repo-config.md',
    target: 'apps/web/static/docs/github-repo-config.md'
  }
];

export function syncDocs({ silent = false } = {}) {
  for (const doc of docs) {
    const source = resolve(rootDir, doc.source);
    const target = resolve(rootDir, doc.target);

    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(source, target);

    if (!silent) {
      console.log(`[sync-docs] ${relative(rootDir, source)} -> ${relative(rootDir, target)}`);
    }
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  syncDocs();
}
