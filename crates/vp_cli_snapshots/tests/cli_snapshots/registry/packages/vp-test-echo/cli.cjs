#!/usr/bin/env node
const { readFileSync } = require('node:fs');

const args = process.argv.slice(2);
const message = args.length ? args.join(' ') : readFileSync(0, 'utf8').trimEnd();
process.stdout.write(`${message}\n`);
