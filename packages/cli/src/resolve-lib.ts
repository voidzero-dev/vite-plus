/**
 * Tsdown tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Tsdown binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Tsdown for running lib.
 *
 * Used for: `vite-plus lib` command
 */

import { DEFAULT_ENVS, resolve } from './utils.js';

/**
 * Resolves the Tsdown binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Tsdown CLI entry point
 *   - envs: Environment variables to set when executing Tsdown
 *
 * Tsdown is a tool that provides a library for building JavaScript/TypeScript libraries.
 */
export async function lib(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Resolve the Tsdown CLI module directly
  const binPath = resolve('tsdown/run');

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      ...DEFAULT_ENVS,
    },
  };
}
