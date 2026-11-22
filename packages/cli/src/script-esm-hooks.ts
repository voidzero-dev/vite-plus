/**
 * Off-thread ESM hooks for `vpx <script>`, registered by `script-register.ts` through
 * `module.register()` where `canRegisterSyncHooks` rules out the in-thread hooks.
 * Adapted from oxc-node's `packages/core/esm.mjs`.
 */

import type { InitializeHook, LoadHook, ResolveHook } from 'node:module';
import { isMainThread, type MessagePort, MessageChannel } from 'node:worker_threads';

import type { FrozenTsconfig } from '../binding/index.js';
import { HELPER_MODULE_NAME, loadOxcNode, resolveWithOxcNode, withFlush } from './script-hooks.ts';

const oxcNode = loadOxcNode();

// Keep the loader worker alive, as upstream does.
if (!isMainThread) {
  new MessageChannel().port1.ref();
}

const resolveHook: ResolveHook = (specifier, context, nextResolve) =>
  resolveWithOxcNode(oxcNode, specifier, context, nextResolve);

const loadHook: LoadHook = (url, context, nextLoad) =>
  oxcNode.load(
    url,
    context as never,
    nextLoad as never,
    HELPER_MODULE_NAME,
  ) as ReturnType<LoadHook>;

/** Under watch mode, the hooks wrapped to report what each call read or looked for. */
let watchHooks: { resolve: ResolveHook; load: LoadHook } | undefined;

/** Receives the frozen loader settings and watch relay from `script-register.ts`. */
export const initialize: InitializeHook<{
  tsconfig: FrozenTsconfig;
  transformDeps: string;
  watchPort?: MessagePort;
}> = (data) => {
  // An absent path explicitly selects discovery, without consulting a stale
  // environment snapshot in the hooks thread.
  oxcNode.initializeLoader(data.tsconfig.path, data.tsconfig.source);
  oxcNode.setTransformDeps(data.transformDeps);
  const port = data.watchPort;
  if (port) {
    port.unref();
    const sink = (message: unknown) => port.postMessage(message);
    watchHooks = {
      resolve: withFlush(resolveHook, sink, oxcNode.takeWatchDependencies),
      load: withFlush(loadHook, sink, oxcNode.takeWatchDependencies),
    };
    oxcNode.enableWatchReport();
  }
};

export const resolve: ResolveHook = (specifier, context, nextResolve) =>
  (watchHooks?.resolve ?? resolveHook)(specifier, context, nextResolve);

export const load: LoadHook = (url, context, nextLoad) =>
  (watchHooks?.load ?? loadHook)(url, context, nextLoad);
