/**
 * Vite tool resolver for the vite-plus CLI.
 *
 * This module exports a function that resolves the Vite binary path
 * using Node.js module resolution. The resolved path is passed back
 * to the Rust core, which then executes Vite with the appropriate
 * command and arguments.
 *
 * Used for: `vite-plus build` and potentially `vite-plus dev` commands
 */

import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

import { cac } from 'cac';

import type { JsCommandContext } from '../binding/index.js';
import { resolveCore } from './resolve-core.ts';
import { DEFAULT_ENVS } from './utils/constants.ts';

export function resolveViteRoot({ cwd, args }: JsCommandContext): string {
  const cli = cac();
  const command = cli.command('[root]').allowUnknownOptions();
  // Match Vite's boolean options so cac does not consume the following root
  // as an option value. Other Vite options accept a required or optional value.
  for (const flag of [
    '--clearScreen',
    '--cors',
    '--strictPort',
    '--force',
    '--experimentalBundle',
    '--emptyOutDir',
    '-w, --watch',
    '--app',
    '-h, --help',
    '-v, --version',
  ]) {
    command.option(flag, '');
  }
  const parsed = cli.parse(['node', 'vite', ...args], { run: false });
  return resolve(cwd, parsed.args[0] ?? '.');
}

/**
 * Resolves the Vite binary path and environment variables.
 *
 * @returns Promise containing:
 *   - binPath: Absolute path to the Vite CLI entry point (vite.js)
 *   - envs: Environment variables to set when executing Vite
 *
 * The CLI and its public re-exports use the same declared `vite` alias.
 */
export async function vite(
  err: Error | null,
  context: JsCommandContext,
): Promise<{
  binPath: string;
  envs: Record<string, string>;
}> {
  if (err) {
    throw err;
  }
  const vitePackagePath = dirname(resolveCore('', resolveViteRoot(context)));
  const binPath = join(vitePackagePath, 'cli.js');
  if (!existsSync(binPath)) {
    throw new Error(`Could not find the bundled Vite CLI at ${binPath}. Run \`vp install\`.`);
  }

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
