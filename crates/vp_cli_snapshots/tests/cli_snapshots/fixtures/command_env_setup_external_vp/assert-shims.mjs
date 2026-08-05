import fs from 'node:fs';
import path from 'node:path';

// Shims of a legacy install are relative links into its own current/bin/vp.
const expected = path.join('..', 'current', 'bin', 'vp');

for (const shim of ['vp', 'node', 'npm', 'npx', 'corepack', 'vpx', 'vpr']) {
  const shimPath = path.join('external', 'bin', shim);
  const target = fs.readlinkSync(shimPath);
  if (target !== expected) {
    throw new Error(`${shim} points to ${target}, expected ${expected}`);
  }
}

console.log('all shims point to the external install');
