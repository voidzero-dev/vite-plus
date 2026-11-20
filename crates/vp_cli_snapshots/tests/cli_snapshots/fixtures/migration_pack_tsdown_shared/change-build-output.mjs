import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';

const config = new URL('./tooling/tsdown-library.ts', import.meta.url);
const source = readFileSync(config, 'utf8');
const updated = source.replace(/viteOutDir\s*=\s*['"]dist['"]/, "viteOutDir = 'vite-out'");
assert.notEqual(updated, source, 'expected the original Vite output directory');
writeFileSync(config, updated);
