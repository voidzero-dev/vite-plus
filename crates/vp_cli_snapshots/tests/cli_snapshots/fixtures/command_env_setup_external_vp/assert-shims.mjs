import fs from 'node:fs';
import path from 'node:path';

const expected = path.resolve('external/vp');
const shims = [
  'vp',
  'node',
  'npm',
  'npx',
  'pnpm',
  'pnpx',
  'yarn',
  'yarnpkg',
  'bun',
  'bunx',
  'vpx',
  'vpr',
];

for (const shim of shims) {
  const shimPath = path.join('home', 'bin', shim);
  const target = fs.readlinkSync(shimPath);
  if (target !== expected) {
    throw new Error(`${shim} points to ${target}, expected ${expected}`);
  }
}

const actualShims = fs.readdirSync(path.join('home', 'bin')).sort();
if (actualShims.join() !== [...shims].sort().join()) {
  throw new Error(`unexpected shims: ${actualShims.join(', ')}`);
}

console.log('all shims point to external vp');
