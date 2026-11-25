#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

const { positionals } = parseArgs({
  allowPositionals: true,
});

const filename = positionals[0];
const script = positionals[1];

if (!filename || !script) {
  console.error('Usage: json-edit <filename> <script>');
  console.error('Example: json-edit package.json \'_.version = "1.2.3"\'');
  process.exit(1);
}

const json: unknown = JSON.parse(readFileSync(filename, 'utf8'));
// oxlint-disable-next-line typescript/no-implied-eval
const func = new Function('_', script + '; return _;');
// oxlint-disable-next-line typescript/no-unsafe-call -- Function has no typed signature for the caller's edit script.
const result: unknown = func(json);

writeFileSync(filename, JSON.stringify(result, null, 2) + '\n', 'utf8');
