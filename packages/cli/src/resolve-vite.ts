import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

import { cac } from 'cac';

import type { JsCommandContext, JsCommandResolvedResult } from '../binding/index.js';
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

/** Resolve the bundled CLI for dev, build, and preview from the selected project root. */
export async function vite(
  err: Error | null,
  context: JsCommandContext,
): Promise<JsCommandResolvedResult> {
  if (err) {
    throw err;
  }
  const vitePackagePath = dirname(resolveCore('', resolveViteRoot(context)));
  const binPath = join(vitePackagePath, 'cli.js');
  if (!existsSync(binPath)) {
    throw new Error(`Could not find the bundled Vite CLI at ${binPath}. Run \`vp install\`.`);
  }

  const envs: Record<string, string> = { ...DEFAULT_ENVS };
  if (process.env.DEBUG_DISABLE_SOURCE_MAP) {
    envs.DEBUG_DISABLE_SOURCE_MAP = process.env.DEBUG_DISABLE_SOURCE_MAP;
  }
  return { binPath, envs };
}
