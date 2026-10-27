import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { source } from './packages/lib/dist/index.mjs';

const workspaceDir = dirname(fileURLToPath(import.meta.url));
assert.equal(source, 'package source');
assert.equal(
  readFileSync(join(workspaceDir, 'packages', 'lib', 'resolved.txt'), 'utf8'),
  'shared config hooks used package cwd\n',
);
assert.equal(existsSync(join(workspaceDir, 'dist')), false);
assert.equal(existsSync(join(workspaceDir, 'configured-root', 'dist')), false);
console.log('shared config keeps plugin root and pack paths in the package');
