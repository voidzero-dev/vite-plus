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

import { DEFAULT_ENVS, resolve } from './utils.js';

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
  const binPath = resolve('oxlint/bin/oxlint');

  const result = {
    binPath,
    // TODO: provide envs inference API
    envs: DEFAULT_ENVS,
  };
  if (process.platform !== 'win32') {
    // @ts-expect-error - OXLINT_TSGOLINT_PATH is not defined in DEFAULT_ENVS
    result.envs.OXLINT_TSGOLINT_PATH = resolve('oxlint-tsgolint/bin/tsgolint.js');
  }
  return result;
}
