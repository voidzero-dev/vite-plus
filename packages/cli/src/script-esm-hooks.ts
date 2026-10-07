/**
 * Off-thread ESM hooks for `vpx <script>` on Node.js < 26.2, registered by
 * `script-register.ts` through `module.register()`. Adapted from oxc-node's
 * `packages/core/esm.mjs`.
 */

import type { LoadHook, ResolveHook } from 'node:module';
import { isMainThread, MessageChannel } from 'node:worker_threads';

import { oxcNode } from '../binding/index.js';
import { HELPERS_PARENT_URL, runtimeHelperSpecifier } from './script-hooks.ts';

// Keep the loader worker alive, as upstream does.
if (!isMainThread) {
  new MessageChannel().port1.ref();
}

export const resolve: ResolveHook = (specifier, context, nextResolve) => {
  const helper = runtimeHelperSpecifier(specifier);
  if (helper) {
    return nextResolve(helper, { ...context, parentURL: HELPERS_PARENT_URL });
  }
  return oxcNode.createResolve(
    { getCurrentDirectory: () => process.cwd() },
    specifier,
    context as never,
    nextResolve as never,
  ) as ReturnType<ResolveHook>;
};

export const load: LoadHook = (url, context, nextLoad) =>
  oxcNode.load(url, context as never, nextLoad as never) as ReturnType<LoadHook>;
