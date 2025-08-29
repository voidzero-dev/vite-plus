/**
 * Oxlint tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the oxlint binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes oxlint for code linting.
 *
 * Used for: `vite-plus lint` command
 *
 * Oxlint is a fast JavaScript/TypeScript linter written in Rust that
 * provides ESLint-compatible linting with significantly better performance.
 */

import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);

/**
 * Resolves the oxlint binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the oxlint binary
 *   - envs: Environment variables to set when executing oxlint
 *
 * The environment variables provide runtime context to oxlint,
 * including Node.js version information and package manager details.
 */
export async function lint(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  // Resolve the oxlint binary directly (it's a native executable)
  const binPath = require.resolve('oxlint/bin/oxlint', {
    paths: [process.cwd(), dirname(fileURLToPath(import.meta.url))],
  });

  return {
    binPath,
    // TODO: provide envs inference API
    envs: {
      // Provide Node.js runtime information for oxlint's telemetry/compatibility
      JS_RUNTIME_VERSION: process.versions.node,
      JS_RUNTIME_NAME: process.release.name,
      // Indicate that vite-plus is the package manager invoking oxlint
      NODE_PACKAGE_MANAGER: 'vite-plus',
    },
  };
}
