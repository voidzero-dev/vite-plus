import { join } from 'node:path';

import type { JsCommandContext, JsCommandResolvedResult } from '../binding/index.js';
import { resolveCore } from './resolve-core.ts';
import { DEFAULT_ENVS } from './utils/constants.ts';

/** Validate the target project's core alias before starting the bundled pack entry. */
export async function pack(
  err: Error | null,
  { cwd }: JsCommandContext,
): Promise<JsCommandResolvedResult> {
  if (err) {
    throw err;
  }
  resolveCore('/pack', cwd);
  return {
    binPath: join(import.meta.dirname, 'pack-bin.js'),
    envs: { ...DEFAULT_ENVS },
  };
}
