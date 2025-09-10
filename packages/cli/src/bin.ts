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
import { lint } from './lint.ts';
import { test } from './test.ts';
import { vite } from './vite.ts';

// Initialize the CLI with tool resolvers
// These functions will be called from Rust when needed
run({
  lint, // Resolves oxlint binary for linting
  vite, // Resolves vite binary for build/dev commands
  test, // Resolves vitest binary for test commands
});
