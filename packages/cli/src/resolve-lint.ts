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

import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { relative } from 'node:path/win32';
import { fileURLToPath } from 'node:url';

import { DEFAULT_ENVS, resolve } from './utils/constants.js';

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
  let oxlintTsgolintPath = resolve('oxlint-tsgolint/bin/tsgolint');
  if (process.platform === 'win32') {
    // On Windows, try .exe first (bun creates .exe), then .cmd (npm/pnpm/yarn create .cmd)
    const localBinDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'node_modules', '.bin');
    const cwdBinDir = join(process.cwd(), 'node_modules', '.bin');
    oxlintTsgolintPath =
      [
        join(localBinDir, 'tsgolint.exe'),
        join(localBinDir, 'tsgolint.cmd'),
        join(cwdBinDir, 'tsgolint.exe'),
        join(cwdBinDir, 'tsgolint.cmd'),
      ].find((p) => existsSync(p)) ?? join(cwdBinDir, 'tsgolint.cmd');
    const relativePath = relative(process.cwd(), oxlintTsgolintPath);
    // Only prepend .\ if it's actually a relative path (not an absolute path returned by relative())
    oxlintTsgolintPath = /^[a-zA-Z]:/.test(relativePath) ? relativePath : `.\\${relativePath}`;
  }
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
