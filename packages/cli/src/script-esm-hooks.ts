/**
 * Off-thread ESM hooks for `vpx <script>` on Node.js < 26, registered by
 * `script-register.ts` through `module.register()`. Adapted from oxc-node's
 * `packages/core/esm.mjs`.
 */

import type { LoadHook, ResolveHook } from 'node:module';
import { isMainThread, MessageChannel } from 'node:worker_threads';

import { oxcNode } from '../binding/index.js';
import { resolveWithOxcNode } from './script-hooks.ts';

// Keep the loader worker alive, as upstream does.
if (!isMainThread) {
  new MessageChannel().port1.ref();
}

export const resolve: ResolveHook = (specifier, context, nextResolve) =>
  resolveWithOxcNode(oxcNode, specifier, context, nextResolve);

export const load: LoadHook = (url, context, nextLoad) =>
  oxcNode.load(url, context as never, nextLoad as never) as ReturnType<LoadHook>;
