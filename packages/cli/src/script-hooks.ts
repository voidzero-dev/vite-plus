/**
 * Shared pieces of the `vpx <script>` loader (see `script-register.ts`).
 */

/** Must match `engines.node` in package.json; `script-hooks.spec.ts` checks it. */
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
export const HELPERS_PARENT_URL = import.meta.url;

const HELPER_REQUIRE = `require("${OXC_NODE_HELPERS}`;
const RUNTIME_HELPER_REQUIRE = `process.getBuiltinModule("node:module").createRequire(${JSON.stringify(HELPERS_PARENT_URL)})("${RUNTIME_HELPERS}`;

/**
 * The CommonJS transform emits `require()` for runtime helpers but leaves
 * `import`/`export` in place, and Node.js runs such a file as an ES module,
 * where `require` is not defined. Reach the helpers in a way both module
 * systems can run, resolved from this package.
 */
export function rewriteHelperRequires(code: string): string {
  return code.replaceAll(HELPER_REQUIRE, RUNTIME_HELPER_REQUIRE);
}
