/**
 * VitePress tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the VitePress binary path
 * to the bundled VitePress in the CLI distribution. The resolved path is
 * passed back to the Rust core, which then executes VitePress with the
 * appropriate command and arguments.
 *
 * Used for: `vite doc` command
 */

import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { DEFAULT_ENVS } from './utils.js';

/**
 * Resolves the VitePress binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the VitePress CLI entry point (vitepress.js)
 *   - envs: Environment variables to set when executing VitePress
 *
 * The function points to the bundled VitePress in the CLI's dist directory.
 */
export async function doc(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // VitePress's CLI binary is located at bin/vitepress.js relative to the package root
  const binPath = join(
    dirname(fileURLToPath(import.meta.url)),
    'vitepress',
    'node',
    'cli.js',
  );

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      ...DEFAULT_ENVS,
    },
  };
}
