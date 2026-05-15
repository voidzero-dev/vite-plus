/**
 * Vitest tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vitest binary path
 * to the vitest package installed alongside the CLI (or in the user's
 * project, resolved from the current working directory first). The
 * resolved path is passed back to the Rust core, which then executes
 * Vitest for running tests.
 *
 * Used for: `vite-plus test` command
 */

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import { DEFAULT_ENVS, resolve } from './utils/constants.ts';

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
 * The function resolves vitest from the user's project first, falling
 * back to the copy installed alongside the CLI.
 */
export async function test(): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  const pkgJsonPath = resolve('vitest/package.json');
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
