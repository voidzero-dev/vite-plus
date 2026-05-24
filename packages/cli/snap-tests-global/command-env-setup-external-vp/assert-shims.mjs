import fs from 'node:fs';
import path from 'node:path';

const expected = path.resolve('external/vp');

for (const shim of ['vp', 'node', 'npm', 'npx', 'vpx', 'vpr']) {
  const shimPath = path.join('home', 'bin', shim);
  const target = fs.readlinkSync(shimPath);
  if (target !== expected) {
    throw new Error(`${shim} points to ${target}, expected ${expected}`);
  }
}

console.log('all shims point to external vp');
