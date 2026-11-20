import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { delimiter, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker } from 'node:worker_threads';

const launcher = process.env.PATH.split(delimiter)
  .map((directory) => resolve(directory, 'vp'))
  .find(existsSync);
assert.ok(launcher, 'expected the project-local vp launcher on PATH');
const worker = new Worker(pathToFileURL(launcher), { argv: process.argv.slice(2) });
const heartbeat = setInterval(() => console.log('worker host alive'), 80);
worker.once('exit', (code) => {
  clearInterval(heartbeat);
  process.exitCode = code;
});
