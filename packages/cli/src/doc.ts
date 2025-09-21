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

import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

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
  const paths = [process.cwd(), dirname(fileURLToPath(import.meta.url))];

  // VitePress's CLI binary is located at bin/vitepress.js relative to the package root
  const pkgJsonPath = require.resolve('vitepress/package.json', {
    paths,
  });
  const binPath = join(dirname(pkgJsonPath), 'bin', 'vitepress.js');

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      // Provide Node.js runtime information for telemetry/compatibility
      JS_RUNTIME_VERSION: process.versions.node,
      JS_RUNTIME_NAME: process.release.name,
      // Indicate that vite-plus is the package manager
      NODE_PACKAGE_MANAGER: 'vite-plus',
    },
  };
}
