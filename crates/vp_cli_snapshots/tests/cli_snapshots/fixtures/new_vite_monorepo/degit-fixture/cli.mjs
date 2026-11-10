#!/usr/bin/env node
import assert from 'node:assert/strict';
import { cpSync, existsSync } from 'node:fs';

// These cases check monorepo creation and catalog migration. Keep the
// downloaded template fixed while running the real package manager and CLI.
const [source, destination, ...extra] = process.argv.slice(2);
assert.equal(source, 'sxzz/tsdown-templates/vite-plus');
assert.ok(destination, 'Missing template destination');
assert.deepEqual(extra, [], 'Unexpected template arguments');
assert.equal(existsSync(destination), false, 'Template destination already exists');
cpSync(new URL('./template/', import.meta.url), destination, { recursive: true });
