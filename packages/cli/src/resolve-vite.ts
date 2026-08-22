/**
 * Vite tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vite binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Vite with the appropriate
 * command and arguments.
 *
 * Used for: `vite-plus build` and potentially `vite-plus dev` commands
 */

import { dirname, join } from 'node:path';

import { DEFAULT_ENVS, resolve } from './utils/constants.ts';
import { checkCoreVersionMatchOnce } from './utils/core-version-guard.ts';

/**
 * Resolves the Vite binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vite CLI entry point (vite.js)
 *   - envs: Environment variables to set when executing Vite
 *
 * The function first tries to resolve vite package, then falls back
 * to vite package (for direct vite installations).
 * It constructs the path to the CLI binary within the resolved package.
 */
export async function vite(
  // Callee-handled NAPI callback: the payload (the command's cwd) is the
  // second argument, after the error slot.
  _err?: unknown,
  taskDir?: string,
): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Fail fast on a `vite` alias that skews from the CLI (see core-version-guard.ts).
  checkCoreVersionMatchOnce(taskDir);

  // Vite's CLI binary is located at bin/vite.js relative to the package root
  const vitePackagePath = dirname(resolve('@voidzero-dev/vite-plus-core'));
  const binPath = join(vitePackagePath, 'cli.js');

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
