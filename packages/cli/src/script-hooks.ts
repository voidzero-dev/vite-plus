/**
 * Shared pieces of the `vpx <script>` loader (see `script-preload.ts`).
 */

import { createRequire } from 'node:module';

import { type WatchSink, watchSink } from '../../../oxc-node/packages/core/watch-report.mjs';

// oxc-node's watch reporting for a loader of its own (`@oxc-node/core/watch-report`),
// bundled from the vendored source, so it matches the binding. `script-preload.ts` also
// imports `@oxc-node/core/signal-ack` this way.
export { createRelay, flush, withFlush } from '../../../oxc-node/packages/core/watch-report.mjs';

type OxcNodeBinding = typeof import('../binding/index.js').oxcNode;

/**
 * The oxc-node hooks compiled into the Vite+ binding. Required, not imported: importing
 * `index.js` makes a `module.register()` hooks thread translate the whole CommonJS
 * binding for ESM, and `module.register()` waits for that (5-13 ms).
 */
export function loadOxcNode(): OxcNodeBinding {
  const binding = createRequire(import.meta.url)(
    '../binding/index.cjs',
  ) as typeof import('../binding/index.js');
  return binding.oxcNode;
}

const WATCH_SINK = Symbol.for('vite-plus:script-watch-sink');

/**
 * Where this thread reports the files watch mode restarts on, or `null` when nobody is
 * watching (see oxc-node's `watchSink()`). It opens a channel, so it runs once per thread:
 * the preload and the `--import` entry are separate bundles and share it here.
 */
export function threadWatchSink(): WatchSink | null {
  const holder = globalThis as { [WATCH_SINK]?: WatchSink | null };
  if (holder[WATCH_SINK] === undefined) {
    holder[WATCH_SINK] = watchSink();
  }
  return holder[WATCH_SINK];
}

/** Must match `engines.node` in package.json; `__tests__/vpx-script.spec.ts` checks it. */
export const SUPPORTED_NODE_RANGE = '^22.18.0 || ^24.11.0 || >=26.0.0';

export function isSupportedNodeVersion(version: string): boolean {
  const [major = 0, minor = 0] = version.split('.').map(Number);
  if (major === 22) {
    return minor >= 18;
  }
  if (major === 24) {
    return minor >= 11;
  }
  return major >= 26;
}

/**
 * Whether to use the in-thread `module.registerHooks()` instead of the off-thread
 * `module.register()`, which needs a hooks thread (about twice the startup time) and
 * which Node.js deprecates from 25.9 (DEP0205).
 *
 * The in-thread hooks need two fixes: a sync resolve hook had its `conditions`
 * overridden (nodejs/node#59011, in 22.19 and 24.5), and a sync load hook could not
 * return a null source for CommonJS (nodejs/node#59929, in 22.22.3 and 24.11.1). tsx
 * switches at the same releases. Upstream oxc-node waits for 26.2 for a third fix,
 * nodejs/node#62920 (also in 24.18.0), which only matters when a load hook returns
 * CommonJS source. These hooks return it only for source another hook supplied, so
 * without that fix `require()` inside such a module can fail.
 */
export function canRegisterSyncHooks(version: string): boolean {
  const [major = 0, minor = 0, patch = 0] = version.split('.').map(Number);
  if (major === 22) {
    return minor > 22 || (minor === 22 && patch >= 3);
  }
  if (major === 24) {
    return minor > 11 || (minor === 11 && patch >= 1);
  }
  return major >= 26;
}

/**
 * The module name oxc-node gives the runtime helpers it injects, passed to its transformer
 * and load hook. Only these hooks emit it, so they redirect only their own helper imports,
 * not `@oxc-node/core/helpers/*` imports that another oxc-node copy produced.
 */
export const HELPER_MODULE_NAME = 'vite-plus:oxc-node';

const OXC_NODE_HELPERS = `${HELPER_MODULE_NAME}/helpers/`;
const RUNTIME_HELPERS = '@oxc-project/runtime/helpers/';

/**
 * oxc-node lowers class fields, static blocks, `using`, and legacy decorators to
 * helper imports. The same helpers ship in `@oxc-project/runtime`, a dependency of
 * `vite-plus`; the hooks resolve them from here, because the user's project usually
 * does not have it.
 */
export function runtimeHelperSpecifier(specifier: string): string | undefined {
  return specifier.startsWith(OXC_NODE_HELPERS)
    ? RUNTIME_HELPERS + specifier.slice(OXC_NODE_HELPERS.length)
    : undefined;
}

/** URL that runtime helper imports resolve from: this package. */
const HELPERS_PARENT_URL = import.meta.url;

interface CommonJsModule {
  _resolveFilename: (
    this: unknown,
    request: string,
    parent: unknown,
    isMain: boolean,
    options?: unknown,
  ) => string;
}

/**
 * CommonJS output (a file without `import`/`export`) loads helpers with `require()`,
 * which never reaches the `module.register()` hooks, while `registerHooks()` hands it
 * to Node.js' CommonJS resolution. Both go through `Module._resolveFilename`, so this
 * wraps it to resolve the helpers from this package, as upstream's `register.mjs` does.
 * The mapped specifier is not a helper specifier, so resolving it falls through.
 */
export function routeHelperRequires(Module: CommonJsModule): void {
  /* oxlint-disable no-underscore-dangle -- Node.js' CommonJS resolver */
  const resolveFilename = Module._resolveFilename;
  const requireFromHere = createRequire(HELPERS_PARENT_URL);
  Module._resolveFilename = function (request, parent, isMain, options) {
    const helper = runtimeHelperSpecifier(request);
    return helper
      ? requireFromHere.resolve(helper)
      : resolveFilename.call(this, request, parent, isMain, options);
  };
  /* oxlint-enable no-underscore-dangle */
}

/**
 * The first line of oxc-node's error for `require()` of an ES module with top-level
 * await, worded as Node.js words it.
 */
const REQUIRE_ASYNC_MODULE_MESSAGE =
  'require() cannot be used on an ESM graph with top-level await. Use import() instead.';

/**
 * Give oxc-node's `require()`-with-top-level-await error the code Node.js uses, which
 * code catches to fall back to `import()`; the binding can only report `GenericFailure`.
 * Mirrors upstream's `register.mjs` (oxc-project/oxc-node#842).
 */
export function withRequireAsyncModuleCode(error: unknown): unknown {
  if (error instanceof Error && error.message.startsWith(REQUIRE_ASYNC_MODULE_MESSAGE)) {
    (error as NodeJS.ErrnoException).code = 'ERR_REQUIRE_ASYNC_MODULE';
  }
  return error;
}

/**
 * Whether this request comes from `require()`. Only `module.registerHooks()` shows
 * `require()` to the hooks; those requests stay on Node.js' CommonJS resolution and
 * the `pirates` hook. A CommonJS context never carries `importAttributes`.
 */
export function isCommonJsRequire(context: { importAttributes?: unknown } | undefined): boolean {
  return context?.importAttributes === undefined;
}

/**
 * The resolve hook for both hook paths: `require()` stays on Node.js' resolution, where
 * `routeHelperRequires` maps runtime helpers (it resolves from the requiring file and
 * ignores a `parentURL`), runtime helper imports resolve from this package, and
 * everything else goes through oxc-node.
 */
export function resolveWithOxcNode<Context extends { importAttributes?: unknown }, Result>(
  oxcNode: OxcNodeBinding,
  specifier: string,
  context: Context,
  nextResolve: (specifier: string, context?: Context) => Result,
): Result {
  if (isCommonJsRequire(context)) {
    return nextResolve(specifier, context);
  }
  const helper = runtimeHelperSpecifier(specifier);
  if (helper) {
    return nextResolve(helper, { ...context, parentURL: HELPERS_PARENT_URL });
  }
  return oxcNode.createResolve(specifier, context as never, nextResolve as never) as Result;
}
