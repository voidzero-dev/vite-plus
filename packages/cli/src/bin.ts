/**
 * Unified entry point for both the local CLI (via bin/vp) and the global CLI (via Rust vp binary).
 *
 * Global commands (create, migrate, --version) are handled by dedicated modules.
 * All other commands are delegated to the Rust core through NAPI bindings, which
 * uses JavaScript tool resolver functions to locate tool binaries.
 */

import { run } from '../binding/index.js';
import { doc } from './resolve-doc.js';
import { fmt } from './resolve-fmt.js';
import { lint } from './resolve-lint.js';
import { pack } from './resolve-pack.js';
import { test } from './resolve-test.js';
import { resolveUniversalViteConfig } from './resolve-vite-config.js';
import { vite } from './resolve-vite.js';

// Parse command line arguments
let args = process.argv.slice(2);

// Transform `vp help [command]` into `vp [command] --help`
if (args[0] === 'help' && args[1]) {
  args = [args[1], '--help', ...args.slice(2)];
  process.argv = process.argv.slice(0, 2).concat(args);
}

const command = args[0];

// Global commands — handled by rolldown-bundled modules in dist/global/
if (command === 'create') {
  await import('./global/create.js');
} else if (command === 'migrate') {
  await import('./global/migrate.js');
} else if (command === '--version' || command === '-V') {
  await import('./global/version.js');
} else {
  // All other commands — delegate to Rust core via NAPI binding
  run({
    lint,
    pack,
    fmt,
    vite,
    test,
    doc,
    resolveUniversalViteConfig,
    args: process.argv.slice(2),
  })
    .then((exitCode) => {
      process.exit(exitCode);
    })
    .catch((err) => {
      console.error('[Vite+] run error:', err);
      process.exit(1);
    });
}
