/**
 * `vpx watch`: oxc-node's watcher (`@oxc-node/cli/watch`), bundled from the vendored
 * source, running the script with the `vpx <script>` loader. Built to
 * `dist/script-watch.js`.
 *
 * The global `vpx` runs `node dist/script-watch.js <args>`, and the project-local bin
 * imports it in process; either way the watcher's arguments are `process.argv.slice(2)`.
 * The watcher owns SIGINT, SIGTERM, SIGHUP, stdin, and the IPC channel until it stops.
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { runWatch } from '../../../oxc-node/packages/cli/src/watch/index.ts';

// The watcher's `--tsconfig` reaches the loader as `OXC_TSCONFIG_PATH`, which an
// inherited `vpx --tsconfig` choice would override.
delete process.env.VP_SCRIPT_TSCONFIG;

const dist = path.dirname(fileURLToPath(import.meta.url));

process.exitCode = await runWatch(process.argv.slice(2), {
  name: 'vpx watch',
  // As in script mode; see `vpx-bin.ts`.
  loaderArgs: [
    '--require',
    path.join(dist, 'script-preload.cjs'),
    '--import',
    new URL('./script-register.js', import.meta.url).href,
  ],
  // The loader's own files and the native binding show up in the module graph.
  ignoreDirs: [path.dirname(dist)],
});
