#!/usr/bin/env node

import assert from 'node:assert';
import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

export function jsonSort() {
  const { positionals } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
  });

  const filename = positionals[0];
  const script = positionals[1];

  if (!filename || !script) {
    console.error('Usage: tool json-sort <filename> <script>');
    console.error("Example: tool json-sort array.json '_.name'");
    process.exit(1);
  }

  const data = JSON.parse(readFileSync(filename, 'utf-8'));
  assert(Array.isArray(data), 'json data must be an array');
  // sort json by script
  const func = new Function('_', `return ${script};`);
  const sortedJson = data.sort((a: any, b: any) => {
    const aValue = func(a);
    const bValue = func(b);
    if (aValue < bValue) {
      return -1;
    }
    if (aValue > bValue) {
      return 1;
    }
    return 0;
  });

  writeFileSync(filename, JSON.stringify(sortedJson, null, 2) + '\n', 'utf-8');
}
