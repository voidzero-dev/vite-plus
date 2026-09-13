#!/usr/bin/env node
const assert = require('node:assert/strict');
const { execSync } = require('node:child_process');

assert.equal(process.version, 'v22.18.0');
assert.equal(execSync('node --version', { encoding: 'utf8' }).trim(), process.version);
const result = JSON.parse(execSync('pnpm --version', { encoding: 'utf8' }));
assert.equal(result.node, process.version);
const npm = execSync('npm --version', { encoding: 'utf8' }).trim();
assert.equal(result.npm, npm);
if (process.argv[2]) assert.equal(result.npm, process.argv[2]);
console.log('The global package and system pnpm use Node 22.18.0');
