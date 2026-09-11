#!/usr/bin/env node
const assert = require('node:assert/strict');
const { execFileSync, execSync } = require('node:child_process');
const path = require('node:path');

assert.equal(process.version, 'v22.18.0');
assert.equal(execFileSync('node', ['--version'], { encoding: 'utf8' }).trim(), process.version);
const shim = path.join(
  process.env.VP_HOME,
  'bin',
  process.platform === 'win32' ? 'node.exe' : 'node',
);
assert.equal(execFileSync(shim, ['--version'], { encoding: 'utf8' }).trim(), process.version);
if (process.argv[2]) {
  const npmVersion = execSync('npm --version', { encoding: 'utf8' }).trim();
  assert.equal(npmVersion, process.argv[2]);
  console.log(`Pinned npm ${npmVersion} is preserved`);
}
console.log('The global package and its Node children use Node 22.18.0');
