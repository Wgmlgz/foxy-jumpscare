import { build } from 'esbuild';
import { cp, mkdir, rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = process.cwd();
const outDir = resolve(root, 'out');

await rm(outDir, { recursive: true, force: true });
await mkdir(outDir, { recursive: true });

await Promise.all([
  cp(resolve(root, 'frontend', 'index.html'), resolve(outDir, 'index.html')),
  cp(resolve(root, 'public', 'media'), resolve(outDir, 'media'), {
    recursive: true,
  }),
]);

await build({
  entryPoints: [resolve(root, 'frontend', 'main.ts')],
  outfile: resolve(outDir, 'main.js'),
  bundle: true,
  minify: true,
  platform: 'browser',
  target: ['es2020'],
  logLevel: 'info',
});
