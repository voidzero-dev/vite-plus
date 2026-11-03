import { mkdirSync, symlinkSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';

// Raw configs use upstream resolution, so expose their direct runner dependency
// in the staged fixture, as the packed case does through its package manager.
const require = createRequire(import.meta.resolve('vite-plus/package.json'));
mkdirSync('node_modules', { recursive: true });
symlinkSync(dirname(require.resolve('vitest/package.json')), resolve('node_modules/vitest'), 'junction');
