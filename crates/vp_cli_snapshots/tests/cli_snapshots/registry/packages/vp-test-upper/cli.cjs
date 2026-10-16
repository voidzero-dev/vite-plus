#!/usr/bin/env node
const { readFileSync } = require('node:fs');

process.stdout.write(`${readFileSync(0, 'utf8').trimEnd().toUpperCase()}\n`);
