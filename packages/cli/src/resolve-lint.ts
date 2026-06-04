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

import { existsSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { DEFAULT_ENVS, resolve } from './utils/constants.ts';

export function resolveWindowsTsgolintExecutable(
  pathCandidates: string[],
  options: {
    exists: (path: string) => boolean;
    getRealpathCandidates?: () => string[];
  },
): string {
  let oxlintTsgolintPath = pathCandidates.find((p) => options.exists(p)) ?? '';
  if (!oxlintTsgolintPath && options.getRealpathCandidates) {
    try {
      oxlintTsgolintPath = options.getRealpathCandidates().find((p) => options.exists(p)) ?? '';
    } catch {
      // realpath failed, fall through to default
    }
  }
  if (!oxlintTsgolintPath) {
    throw new Error(
      'Unable to resolve oxlint-tsgolint executable, tried:\n' +
        pathCandidates.map((path) => `- ${path}`).join('\n'),
    );
  }
  return oxlintTsgolintPath;
}

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
    const scriptDir = dirname(fileURLToPath(import.meta.url));
    const localBinDir = join(scriptDir, '..', 'node_modules', '.bin');
    const oxlintTsgolintPackagePath = dirname(dirname(oxlintTsgolintPath));
    const projectBinDir = join(oxlintTsgolintPackagePath, '..', '.bin');
    const pathCandidates = [
      join(localBinDir, 'tsgolint.exe'),
      join(localBinDir, 'tsgolint.cmd'),
      join(projectBinDir, 'tsgolint.exe'),
      join(projectBinDir, 'tsgolint.cmd'),
    ];
    oxlintTsgolintPath = resolveWindowsTsgolintExecutable(pathCandidates, {
      exists: existsSync,
      // Bun stores packages in .bun/ cache dirs where the symlinked paths above won't match.
      getRealpathCandidates: () => {
        const realPkgDir = realpathSync(join(scriptDir, '..'));
        const realBinDir = join(dirname(realPkgDir), '.bin');
        return [join(realBinDir, 'tsgolint.exe'), join(realBinDir, 'tsgolint.cmd')];
      },
    });
    // Keep the resolved absolute path. oxlint may be spawned with a different cwd than
    // this launcher (e.g. the workspace package dir under `vp run -r`), where a path made
    // relative to the launcher's process.cwd() would resolve against the wrong base
    // directory and fail (e.g. pnpm's `.pnpm` only exists at the monorepo root).
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
