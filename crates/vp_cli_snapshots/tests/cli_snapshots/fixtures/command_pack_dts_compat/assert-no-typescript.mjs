import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const cliRequire = createRequire(import.meta.resolve('vite-plus/package.json'));
const coreRequire = createRequire(cliRequire.resolve('vite/package.json'));

assert.throws(() => coreRequire.resolve('typescript'), { code: 'MODULE_NOT_FOUND' });
assert.equal(
  coreRequire('@typescript/native-preview/package.json').version,
  '7.0.0-dev.20260707.2',
);
