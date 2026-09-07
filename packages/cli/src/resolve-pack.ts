/**
 * Tsdown tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Tsdown binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Tsdown for running pack.
 *
 * Used for: `vite-plus pack` command
 */

import { join } from 'node:path';

import type { JsCommandContext } from '../binding/index.js';
import { resolveCore } from './resolve-core.ts';
import { DEFAULT_ENVS } from './utils/constants.ts';

/**
 * Resolves the Tsdown binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Tsdown CLI entry point
 *   - envs: Environment variables to set when executing Tsdown
 *
 * Tsdown is a tool that provides a library for building JavaScript/TypeScript libraries.
 */
export async function pack(
  err: Error | null,
  { cwd }: JsCommandContext,
): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  if (err) {
    throw err;
  }
  resolveCore('/pack', cwd);
  // Resolve the bundled Tsdown CLI
  const binPath = join(import.meta.dirname, 'pack-bin.js');

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      ...DEFAULT_ENVS,
    },
  };
}
