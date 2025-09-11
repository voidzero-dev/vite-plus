/**
 * Oxfmt tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the oxfmt binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes oxfmt for code formatting.
 *
 * Used for: `vite-plus fmt` command
 *
 * Oxfmt is a fast JavaScript/TypeScript formatter written in Rust that
 * provides high-performance code formatting capabilities.
 */

import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

/**
 * Resolves the oxfmt binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the oxfmt binary
 *   - envs: Environment variables to set when executing oxfmt
 *
 * The environment variables provide runtime context to oxfmt,
 * including Node.js version information and package manager details.
 */
export async function fmt(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Resolve the oxfmt binary directly (it's a native executable)
  const binPath = require.resolve('oxfmt/bin/oxfmt', {
    paths: [process.cwd(), dirname(fileURLToPath(import.meta.url))],
  });

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      // Provide Node.js runtime information for oxfmt's telemetry/compatibility
      JS_RUNTIME_VERSION: process.versions.node,
      JS_RUNTIME_NAME: process.release.name,
      // Indicate that vite-plus is the package manager invoking oxfmt
      NODE_PACKAGE_MANAGER: 'vite-plus',
    },
  };
}