/**
 * Vitest tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vitest binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Vitest for running tests.
 *
 * Used for: `vite-plus test` command
 */

import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

/**
 * Resolves the Vitest binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vitest CLI entry point (vitest.mjs)
 *   - envs: Environment variables to set when executing Vitest
 *
 * Vitest is Vite's testing framework that provides a Jest-compatible
 * testing experience with Vite's fast HMR and transformation pipeline.
 */
export async function test(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Resolve the Vitest CLI module directly
  const binPath = require.resolve('vitest/vitest.mjs', {
    paths: [process.cwd(), dirname(fileURLToPath(import.meta.url))],
  });

  return {
    binPath,
    // Pass through source map debugging environment variable if set
    envs: process.env.DEBUG_DISABLE_SOURCE_MAP
      ? {
        DEBUG_DISABLE_SOURCE_MAP: process.env.DEBUG_DISABLE_SOURCE_MAP,
      }
      : {},
  };
}
