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

import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);

/**
 * Resolves the Vite binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vite CLI entry point (vite.js)
 *   - envs: Environment variables to set when executing Vite
 *
 * The function uses require.resolve to find the vite package installation,
 * then constructs the path to the CLI binary within the package.
 */
export async function vite(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Find the vite package.json to locate the installation directory
  const pkgJsonPath = require.resolve('vite/package.json');
  // Vite's CLI binary is located at bin/vite.js relative to the package root
  const binPath = join(dirname(pkgJsonPath), 'bin', 'vite.js');

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
