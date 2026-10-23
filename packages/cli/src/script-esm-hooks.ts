/**
 * Off-thread ESM hooks for `vpx <script>`, registered by `script-register.ts` through
 * `module.register()` where `canRegisterSyncHooks` rules out the in-thread hooks.
 * Adapted from oxc-node's `packages/core/esm.mjs`.
 */

import type { InitializeHook, LoadHook, ResolveHook } from 'node:module';
import { isMainThread, type MessagePort, MessageChannel } from 'node:worker_threads';

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

/** Receives the port `script-register.ts` relays watch reports through. */
export const initialize: InitializeHook<
  { tsconfigPath?: string; watchPort?: MessagePort } | undefined
> = (data) => {
  // Native loader state belongs to a thread. Copy the registering thread's choice
  // explicitly, including discovery when its environment cleared a parent config.
  oxcNode.initializeLoader(data?.tsconfigPath);
  oxcNode.setTransformDeps('typescript');
  const port = data?.watchPort;
  if (port) {
    port.unref();
    const sink = (message: unknown) => port.postMessage(message);
    watchHooks = {
      resolve: withFlush(resolveHook, sink, oxcNode.takeWatchDependencies),
      load: withFlush(loadHook, sink, oxcNode.takeWatchDependencies),
    };
  }
};

export const resolve: ResolveHook = (specifier, context, nextResolve) =>
  (watchHooks?.resolve ?? resolveHook)(specifier, context, nextResolve);

export const load: LoadHook = (url, context, nextLoad) =>
  (watchHooks?.load ?? loadHook)(url, context, nextLoad);
