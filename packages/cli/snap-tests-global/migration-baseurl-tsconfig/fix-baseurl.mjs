#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const target = process.argv.at(-1);
if (!target || target.startsWith('-')) {
  process.exit(1);
}

const filePath = path.resolve(process.cwd(), target);
const text = fs.readFileSync(filePath, 'utf8');
fs.writeFileSync(filePath, text.replace(/\n\s*"baseUrl": "\.",?/, ''));
