#!/usr/bin/env node
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const directoryIndex = args.indexOf('--directory');
const directory = directoryIndex === -1 ? 'output' : args[directoryIndex + 1];

mkdirSync(directory, { recursive: true });
writeFileSync(
  path.join(directory, 'package.json'),
  `${JSON.stringify({ name: directory, version: '0.0.0', private: true }, null, 2)}\n`,
);
writeFileSync(path.join(directory, 'template-args.json'), `${JSON.stringify(args, null, 2)}\n`);
