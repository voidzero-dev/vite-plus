/**
 * Vitest tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vitest binary path
 * to the bundled Vitest in the CLI distribution. The resolved path is
 * passed back to the Rust core, which then executes Vitest for running tests.
 *
 * Used for: `vite-plus test` command
 */

import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { DEFAULT_ENVS } from './utils.js';

/**
 * Resolves the Vitest binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vitest CLI entry point (vitest.mjs)
 *   - envs: Environment variables to set when executing Vitest
 *
 * Vitest is Vite's testing framework that provides a Jest-compatible
 * testing experience with Vite's fast HMR and transformation pipeline.
 * The function points to the bundled Vitest in the CLI's dist directory.
 */
export async function test(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const binPath = join(dirname(fileURLToPath(import.meta.url)), 'vitest', 'vitest.mjs');

  return {
    binPath,
    // Pass through source map debugging environment variable if set
    envs: process.env.DEBUG_DISABLE_SOURCE_MAP
      ? {
        ...DEFAULT_ENVS,
        DEBUG_DISABLE_SOURCE_MAP: process.env.DEBUG_DISABLE_SOURCE_MAP,
      }
      : {
        ...DEFAULT_ENVS,
      },
  };
}
