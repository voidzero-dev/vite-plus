import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);
const sourcePackageDir = path.dirname(require.resolve('vite-plus/package.json'));
const localPackageDir = path.join('node_modules', 'vite-plus');

fs.mkdirSync('node_modules', { recursive: true });
fs.cpSync(sourcePackageDir, localPackageDir, { recursive: true, dereference: true });

const fakeVitestDir = path.join('node_modules', 'vitest');
fs.mkdirSync(fakeVitestDir, { recursive: true });
fs.writeFileSync(
  path.join(fakeVitestDir, 'package.json'),
  JSON.stringify({
    name: 'vitest',
    version: '0.1.24',
    type: 'module',
    exports: {
      './config': './config.js',
      './package.json': './package.json',
    },
  }),
);
fs.writeFileSync(
  path.join(fakeVitestDir, 'config.js'),
  "throw new Error('stale Vitest alias was loaded before command dispatch');\n",
);

// This file only prepares node_modules. Remove the staged copy so migration's
// source-import pass measures the project fixture rather than its test harness.
fs.rmSync(new URL(import.meta.url));
