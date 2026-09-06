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

import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';

import { resolveCore } from './resolve-core.ts';
import { DEFAULT_ENVS } from './utils/constants.ts';

/**
 * Resolves the Vite binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vite CLI entry point (vite.js)
 *   - envs: Environment variables to set when executing Vite
 *
 * The CLI and its public re-exports use the same declared `vite` alias.
 */
export async function vite(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const vitePackagePath = dirname(resolveCore());
  const binPath = join(vitePackagePath, 'cli.js');
  if (!existsSync(binPath)) {
    throw new Error(`Could not find the bundled Vite CLI at ${binPath}. Run \`vp install\`.`);
  }

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
