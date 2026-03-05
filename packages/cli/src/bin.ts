/**
 * Unified entry point for both the local CLI (via bin/vp) and the global CLI (via Rust vp binary).
 *
 * Global commands (create, migrate, config, mcp, staged, --version) are handled by rolldown-bundled modules.
 * All other commands are delegated to the Rust core through NAPI bindings, which
 * uses JavaScript tool resolver functions to locate tool binaries.
 *
 * When called from the global CLI, the Rust binary resolves the project's local
 * vite-plus installation using oxc_resolver and runs its dist/bin.js directly.
 * If no local installation is found, this global dist/bin.js is used as fallback.
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
// These modules only exist after rolldown bundles them, so TS cannot resolve them.
if (command === 'create') {
  // @ts-ignore — rolldown output
  await import('./global/create.js');
} else if (command === 'migrate') {
  // @ts-ignore — rolldown output
  await import('./global/migrate.js');
} else if (command === 'config') {
  // @ts-ignore — rolldown output
  await import('./global/config.js');
} else if (command === 'mcp') {
  // @ts-ignore — rolldown output
  await import('./global/mcp.js');
} else if (command === '--version' || command === '-V') {
  // @ts-ignore — rolldown output
  await import('./global/version.js');
} else if (command === 'staged') {
  // @ts-ignore — rolldown output
  await import('./global/staged.js');
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
