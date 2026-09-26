#!/usr/bin/env node
const assert = require('node:assert/strict');
const { existsSync, readFileSync } = require('node:fs');
const { dirname, join } = require('node:path');

// Exercise the installed CLI, including a transitive package's build output.
const parent = existsSync(join(__dirname, 'postinstall-ran')) ? 'ran' : 'skipped';
const child = existsSync(join(dirname(require.resolve('native-addon/package.json')), 'postinstall-ran')) ? 'ran' : 'skipped';
assert.equal(parent, process.argv[2]);
assert.equal(child, process.argv[3]);
if (parent === 'ran') assert.equal(readFileSync(join(__dirname, 'postinstall-ran'), 'utf8'), process.version);
if (child === 'ran') assert.equal(readFileSync(join(dirname(require.resolve('native-addon/package.json')), 'postinstall-ran'), 'utf8'), process.version);
console.log(`my-cli: ${parent}; native-addon: ${child}`);
