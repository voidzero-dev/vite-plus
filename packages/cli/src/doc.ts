/**
 * VitePress tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the VitePress binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes VitePress with the appropriate
 * command and arguments.
 *
 * Used for: `vite doc` command
 */

import { dirname, join } from 'node:path';
import { DEFAULT_ENVS, resolve } from './utils.js';

/**
 * Resolves the VitePress binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the VitePress CLI entry point (vitepress.js)
 *   - envs: Environment variables to set when executing VitePress
 *
 * The function resolves the vitepress package and constructs the path
 * to the CLI binary within the resolved package.
 */
export async function doc(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // VitePress's CLI binary is located at bin/vitepress.js relative to the package root
  const pkgJsonPath = resolve('vitepress/package.json');
  const binPath = join(dirname(pkgJsonPath), 'bin', 'vitepress.js');

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      ...DEFAULT_ENVS,
    },
  };
}
