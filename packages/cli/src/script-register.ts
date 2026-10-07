/**
 * `--import` entry for `vpx <script>`.
 *
 * Registers the oxc-node loader hooks that are compiled into the Vite+ native binding
 * (exported as `oxcNode`). Adapted from oxc-node's `packages/core/register.mjs`; keep
 * them in sync when bumping the vendored oxc-node in `.upstream-versions.json`.
 */

import * as NodeModule from 'node:module';

import { addHook } from 'pirates';

import {
  HELPERS_PARENT_URL,
  isSupportedNodeVersion,
  rewriteHelperRequires,
  runtimeHelperSpecifier,
  SUPPORTED_NODE_RANGE,
} from './script-hooks.ts';

if (!isSupportedNodeVersion(process.versions.node)) {
  process.stderr.write(
    `error: Running scripts with vpx requires Node.js ${SUPPORTED_NODE_RANGE} (current: ${process.version})\n`,
  );
  process.exit(1);
}

// Imported after the version check so the native addon never loads on an
// unsupported Node.js.
const { oxcNode } = await import('../binding/index.js');

// Destructure from the namespace: these APIs are missing on older Node.js releases.
const { register, registerHooks, setSourceMapsSupport } = NodeModule as Partial<typeof NodeModule>;

if (typeof setSourceMapsSupport === 'function') {
  setSourceMapsSupport(true, { nodeModules: true, generatedCode: true });
} else if (typeof process.setSourceMapsEnabled === 'function') {
  process.setSourceMapsEnabled(true);
}

const EXTENSIONS = ['.js', '.jsx', '.ts', '.tsx', '.mjs', '.mts', '.cjs', '.cts', '.es6', '.es'];
const SOURCEMAP_PREFIX = '\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,';

const transformer = new oxcNode.OxcTransformer(process.cwd());

addHook(
  (code, filename) => {
    const output = transformer.transform(filename, code);
    const source = rewriteHelperRequires(output.source());
    const sourceMap = output.sourceMap();
    return sourceMap
      ? source + SOURCEMAP_PREFIX + Buffer.from(sourceMap, 'utf8').toString('base64')
      : source;
  },
  { exts: EXTENSIONS },
);

/**
 * Whether this request comes from `require()`. Only `module.registerHooks()` shows
 * `require()` to the hooks; those requests stay on Node.js' CommonJS resolution and
 * the `pirates` hook above. A CommonJS context never carries `importAttributes`.
 */
function isCommonJsRequire(context: { importAttributes?: unknown } | undefined): boolean {
  return context?.importAttributes === undefined;
}

const resolve: NodeModule.ResolveHookSync = (specifier, context, nextResolve) => {
  const helper = runtimeHelperSpecifier(specifier);
  if (helper) {
    return nextResolve(helper, { ...context, parentURL: HELPERS_PARENT_URL });
  }
  if (isCommonJsRequire(context)) {
    return nextResolve(specifier, context);
  }
  return oxcNode.createResolve(
    { getCurrentDirectory: () => process.cwd() },
    specifier,
    context as never,
    nextResolve as never,
  ) as NodeModule.ResolveFnOutput;
};

const load: NodeModule.LoadHookSync = (url, context, nextLoad) => {
  if (isCommonJsRequire(context)) {
    return nextLoad(url, context);
  }
  const result = oxcNode.load(
    url,
    context as never,
    nextLoad as never,
  ) as NodeModule.LoadFnOutput & { responseURL?: string };
  // Leave CommonJS to the CommonJS loader, which compiles it through `pirates` with
  // an accurate source map. A null source keeps `require()` inside such a module
  // working on every runtime (nodejs/node#62920).
  if (result.format === 'commonjs') {
    // `@types/node` does not model the null source Node.js accepts for CommonJS.
    return {
      format: 'commonjs',
      source: null,
      responseURL: result.responseURL ?? url,
    } as unknown as NodeModule.LoadFnOutput;
  }
  return result;
};

/**
 * `module.registerHooks()` is complete from Node.js 26.2: before it, a sync resolve
 * hook had its `conditions` overridden (nodejs/node#59011) and `require()` inside an
 * imported CommonJS module could short-circuit (nodejs/node#62920).
 */
function canRegisterSyncHooks(): boolean {
  if (typeof registerHooks !== 'function') {
    return false;
  }
  const [major = 0, minor = 0] = process.versions.node.split('.').map(Number);
  return major > 26 || (major === 26 && minor >= 2);
}

if (canRegisterSyncHooks()) {
  registerHooks!({ resolve, load });
} else {
  register!(new URL('./script-esm-hooks.js', import.meta.url));
}
