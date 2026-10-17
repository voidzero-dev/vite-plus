/**
 * Off-thread ESM hooks for `vpx <script>`, registered by `script-register.ts` through
 * `module.register()` where `canRegisterSyncHooks` rules out the in-thread hooks.
 * Adapted from oxc-node's `packages/core/esm.mjs`.
 */

import type { LoadHook, ResolveHook } from 'node:module';
import { createRequire } from 'node:module';
import { isMainThread, MessageChannel } from 'node:worker_threads';

import { HELPER_MODULE_NAME, resolveWithOxcNode } from './script-hooks.ts';

// Required, not imported: importing `index.js` makes this thread translate the whole
// CommonJS binding for ESM, and `module.register()` waits for that (5-13 ms).
const { oxcNode } = createRequire(import.meta.url)(
  '../binding/index.cjs',
) as typeof import('../binding/index.js');

// Keep the loader worker alive, as upstream does.
if (!isMainThread) {
  new MessageChannel().port1.ref();
}

export const resolve: ResolveHook = (specifier, context, nextResolve) =>
  resolveWithOxcNode(oxcNode, specifier, context, nextResolve);

export const load: LoadHook = (url, context, nextLoad) =>
  oxcNode.load(
    url,
    context as never,
    nextLoad as never,
    HELPER_MODULE_NAME,
  ) as ReturnType<LoadHook>;
