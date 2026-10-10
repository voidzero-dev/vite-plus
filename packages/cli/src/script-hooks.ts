/**
 * Shared pieces of the `vpx <script>` loader (see `script-preload.ts`).
 */

type OxcNodeBinding = typeof import('../binding/index.js').oxcNode;

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
 * `module.register()`, which Node.js deprecates from 25.9 (DEP0205).
 *
 * Upstream oxc-node waits for Node.js 26.2 because of two `registerHooks()` defects:
 * a sync resolve hook had its `conditions` overridden (nodejs/node#59011, fixed in
 * 22.19 and 24.5) and `require()` inside an imported CommonJS module short-circuited
 * when a load hook returned its source (nodejs/node#62920, fixed in 26.2). These hooks
 * never return source for CommonJS, so only the first defect matters, and every
 * Node.js 26 release has its fix. 22.x and 24.x keep `module.register()`, which they do
 * not deprecate.
 */
export function canRegisterSyncHooks(version: string): boolean {
  const [major = 0] = version.split('.').map(Number);
  return major >= 26;
}

const OXC_NODE_HELPERS = '@oxc-node/core/helpers/';
const RUNTIME_HELPERS = '@oxc-project/runtime/helpers/';

/**
 * oxc-node lowers class fields, static blocks, `using`, and legacy decorators to
 * helper imports from `@oxc-node/core/helpers/*`. Those resolve from the user's file,
 * where `@oxc-node/core` is usually not installed. The same helpers ship in
 * `@oxc-project/runtime`, a dependency of `vite-plus`; the hooks resolve them from here.
 */
export function runtimeHelperSpecifier(specifier: string): string | undefined {
  return specifier.startsWith(OXC_NODE_HELPERS)
    ? RUNTIME_HELPERS + specifier.slice(OXC_NODE_HELPERS.length)
    : undefined;
}

/** URL that runtime helper imports resolve from: this package. */
const HELPERS_PARENT_URL = import.meta.url;

const HELPER_REQUIRE = `require("${OXC_NODE_HELPERS}`;
const RUNTIME_HELPER_REQUIRE = `process.getBuiltinModule("node:module").createRequire(${JSON.stringify(HELPERS_PARENT_URL)})("${RUNTIME_HELPERS}`;

/**
 * CommonJS output (a file without `import`/`export`) loads helpers with `require()`,
 * which the `module.register()` hooks on Node.js 22 and 24 never see. Load them from
 * this package instead of the user's file.
 */
export function rewriteHelperRequires(code: string): string {
  return code.replaceAll(HELPER_REQUIRE, RUNTIME_HELPER_REQUIRE);
}

/**
 * Whether this request comes from `require()`. Only `module.registerHooks()` shows
 * `require()` to the hooks; those requests stay on Node.js' CommonJS resolution and
 * the `pirates` hook. A CommonJS context never carries `importAttributes`.
 */
export function isCommonJsRequire(context: { importAttributes?: unknown } | undefined): boolean {
  return context?.importAttributes === undefined;
}

// Only the WASI build of oxc-node reads these options; they match upstream's hooks.
const RESOLVE_OPTIONS = { getCurrentDirectory: () => process.cwd() };

/**
 * The resolve hook for both hook paths: runtime helpers resolve from this package,
 * `require()` stays on Node.js' resolution, and everything else goes through oxc-node.
 */
export function resolveWithOxcNode<Context extends { importAttributes?: unknown }, Result>(
  oxcNode: OxcNodeBinding,
  specifier: string,
  context: Context,
  nextResolve: (specifier: string, context?: Context) => Result,
): Result {
  const helper = runtimeHelperSpecifier(specifier);
  if (helper) {
    return nextResolve(helper, { ...context, parentURL: HELPERS_PARENT_URL });
  }
  if (isCommonJsRequire(context)) {
    return nextResolve(specifier, context);
  }
  return oxcNode.createResolve(
    RESOLVE_OPTIONS,
    specifier,
    context as never,
    nextResolve as never,
  ) as Result;
}
