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

import { dirname, join } from 'node:path';

import { DEFAULT_ENVS, resolve } from './utils/constants.ts';
import { resolveTsgolintExecutable } from './utils/tsgolint-path.ts';

export { resolveWindowsTsgolintExecutable } from './utils/tsgolint-path.ts';

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
  // Resolve the oxlint package path first, then navigate to the bin file.
  // The bin/oxlint subpath is not exported in package.json exports, so we
  // resolve the main entry point and derive the bin path from it.
  // resolve('oxlint') returns .../oxlint/dist/index.js, so we need to go up
  // two directories (past 'dist') to reach the package root.
  const oxlintMainPath = resolve('oxlint');
  const oxlintPackageRoot = dirname(dirname(oxlintMainPath));
  const binPath = join(oxlintPackageRoot, 'bin', 'oxlint');
  const oxlintTsgolintPath = resolveTsgolintExecutable(
    resolve('oxlint-tsgolint/bin/tsgolint'),
    import.meta.url,
  );
  const result = {
    binPath,
    // TODO: provide envs inference API
    envs: {
      ...DEFAULT_ENVS,
      OXLINT_TSGOLINT_PATH: oxlintTsgolintPath,
    },
  };
  return result;
}
