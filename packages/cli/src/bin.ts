/**
 * Entry point for the vite-plus CLI.
 *
 * This file initializes the CLI by passing JavaScript tool resolver functions
 * to the Rust core through NAPI bindings. Each resolver function is responsible
 * for locating the binary path of its respective tool using Node.js module resolution.
 *
 * The Rust core will call these functions when it needs to execute the corresponding
 * tools (e.g., when running `vite-plus build`, it calls the vite resolver).
 */

import { run } from '../binding/index.js';
import { doc } from './resolve-doc.js';
import { fmt } from './resolve-fmt.js';
import { lib } from './resolve-lib.js';
import { lint } from './resolve-lint.js';
import { test } from './resolve-test.js';
import { vite } from './resolve-vite.js';

async function resolveUniversalViteConfig(err: null | Error, viteConfigCwd: string) {
  if (err) {
    throw err;
  }
  try {
    const { resolveConfig } = await import('./config.js');
    const config = await resolveConfig({ root: viteConfigCwd }, 'build');

    return Promise.resolve(JSON.stringify({
      lint: config.lint,
      fmt: config.fmt,
    }));
  } catch (err) {
    console.error('[vite+] resolve universal vite config error:', err);
    throw err;
  }
}

// Initialize the CLI with tool resolvers
// These functions will be called from Rust when needed
run({
  lint, // Resolves oxlint binary for linting
  lib, // Resolves tsdown binary for lib bundling
  fmt, // Resolves oxfmt binary for formatting
  vite, // Resolves vite binary for build/dev commands
  test, // Resolves vitest binary for test commands
  doc, // Resolves vitepress binary for doc commands
  resolveUniversalViteConfig,
}).then((exitCode) => {
  process.exit(exitCode);
})
  .catch((err) => {
    console.error('[vite+] run error:', err);
    process.exit(1);
  });
