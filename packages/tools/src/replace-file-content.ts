#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const filename = process.argv[2];
const searchValue = process.argv[3];
const newValue = process.argv[4];

if (!filename || !searchValue || !newValue) {
  console.error('Usage: replace-file-content <filename> <searchValue> <newValue>');
  console.error('Example: replace-file-content example.toml \'version = "0.0.0"\' \'version = "0.0.1"\'');
  process.exit(1);
}

const filepath = path.resolve(filename);
const content = readFileSync(filepath, 'utf-8');
const newContent = content.replace(searchValue, newValue);
writeFileSync(filepath, newContent, 'utf-8');
