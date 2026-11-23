import { Worker } from 'node:worker_threads';
import { pathToFileURL } from 'node:url';

import { localLauncher } from './helpers.mjs';

const worker = new Worker(pathToFileURL(localLauncher()), { argv: process.argv.slice(2) });
const heartbeat = setInterval(() => console.log('worker host alive'), 80);
worker.once('exit', (code) => {
  clearInterval(heartbeat);
  process.exitCode = code;
});
