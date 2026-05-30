/**
 * Vitest tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vitest binary path
 * to the vitest package shipped with the CLI (falling back to the user's
 * project copy only if the bundled one is unreachable). The resolved path
 * is passed back to the Rust core, which then executes Vitest for running
 * tests.
 *
 * Used for: `vite-plus test` command
 */

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import { DEFAULT_ENVS, resolveBundled } from './utils/constants.ts';

interface VitestPackageJson {
  bin?: string | Record<string, string>;
}

/**
 * Resolves the Vitest binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vitest CLI entry point (vitest.mjs)
 *   - envs: Environment variables to set when executing Vitest
 *
 * Vitest is Vite's testing framework that provides a Jest-compatible
 * testing experience with Vite's fast HMR and transformation pipeline.
 * The function resolves the bundled vitest shipped with the CLI first,
 * so the runner matches the Vitest that `vite-plus/test*` imports resolve
 * to; it falls back to the project copy only if the bundled one is
 * unreachable. See `resolveBundled` for the rationale (avoiding dual-copy
 * Vitest internal-state / mock-hoisting mismatches).
 */
export async function test(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const pkgJsonPath = resolveBundled('vitest/package.json');
  const pkgRoot = dirname(pkgJsonPath);
  const pkgJson = JSON.parse(readFileSync(pkgJsonPath, 'utf-8')) as VitestPackageJson;
  const binRel = typeof pkgJson.bin === 'string' ? pkgJson.bin : pkgJson.bin?.vitest;
  if (!binRel) {
    throw new Error(`Could not find 'vitest' bin entry in ${pkgJsonPath}`);
  }
  const binPath = join(pkgRoot, binRel);

  return {
    binPath,
    // Pass through source map debugging environment variable if set
    envs: process.env.DEBUG_DISABLE_SOURCE_MAP
      ? {
          ...DEFAULT_ENVS,
          DEBUG_DISABLE_SOURCE_MAP: process.env.DEBUG_DISABLE_SOURCE_MAP,
        }
      : {
          ...DEFAULT_ENVS,
        },
  };
}
