/**
 * Tsdown tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Tsdown binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Tsdown for running lib.
 *
 * Used for: `vite-plus lib` command
 */

import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

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
  const binPath = require.resolve('tsdown/run', {
    paths: [process.cwd(), dirname(fileURLToPath(import.meta.url))],
  });

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      // Provide Node.js runtime information for oxfmt's telemetry/compatibility
      JS_RUNTIME_VERSION: process.versions.node,
      JS_RUNTIME_NAME: process.release.name,
      // Indicate that vite-plus is the package manager invoking tsdown
      NODE_PACKAGE_MANAGER: 'vite-plus',
    },
  };
}
