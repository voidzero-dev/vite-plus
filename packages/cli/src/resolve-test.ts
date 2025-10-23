/**
 * Vitest tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vitest binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Vitest for running tests.
 *
 * Used for: `vite-plus test` command
 */

import { dirname, join } from 'node:path';
import { DEFAULT_ENVS, resolve } from './utils.js';

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
  // try to resolve vitest package.json
  const pkgJsonPath = resolve('vitest/package.json');
  // vitest's CLI binary is located at vitest.mjs relative to the package root
  const binPath = join(dirname(pkgJsonPath), 'vitest.mjs');

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
