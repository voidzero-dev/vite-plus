#!/usr/bin/env node
const assert = require('node:assert/strict');
const { existsSync } = require('node:fs');
const { join } = require('node:path');

const state = existsSync(join(__dirname, 'postinstall-ran')) ? 'ran' : 'skipped';
assert.equal(state, process.argv[2]);
console.log(`postinstall: ${state}`);
