import fs from 'node:fs';
import path from 'node:path';

const expected = path.resolve('external/bin/vp');
const shims = [
  'vp',
  'node',
  'npm',
  'npx',
  'pnpm',
  'pnpx',
  'pn',
  'pnx',
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

const lockFile = '.vp-global-bin.lock';
if (!fs.lstatSync(path.join('home', 'bin', lockFile)).isFile()) {
  throw new Error('global bin lock must be a regular file');
}

const binEntries = fs.readdirSync(path.join('home', 'bin')).sort();
if (binEntries.join() !== [...shims, lockFile].sort().join()) {
  throw new Error(`unexpected bin entries: ${binEntries.join(', ')}`);
}

console.log('all shims point to external vp');
