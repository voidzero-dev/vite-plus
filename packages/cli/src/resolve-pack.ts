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

import { createToolResolution, type ToolResolution } from './utils/tool-resolution.ts';

/**
 * Resolves the Tsdown binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Tsdown CLI entry point
 *   - envs: Environment variables to set when executing Tsdown
 *
 * Tsdown is a tool that provides a library for building JavaScript/TypeScript libraries.
 */
export async function pack(): Promise<ToolResolution> {
  // Resolve the bundled Tsdown CLI
  const binPath = join(import.meta.dirname, 'pack-bin.js');

  return createToolResolution(binPath);
}
