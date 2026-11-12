/**
 * `--require` entry for `vpx <script>`, built to `dist/script-preload.cjs`.
 *
 * The CLIs pass `--require <dist/script-preload.cjs> --import <dist/script-register.js>`.
 * This preload runs first, before any user `--require` or `--import`, so a
 * `--require ./setup.ts` preload works too. It checks the Node.js version, installs the
 * CommonJS hook, and registers the in-thread ESM hooks where `canRegisterSyncHooks`
 * allows; elsewhere `script-register.js` registers off-thread ESM hooks instead.
 *
 * It uses the oxc-node hooks compiled into the Vite+ native binding (exported as
 * `oxcNode`). Adapted from oxc-node's `packages/core/register.mjs`; keep them in sync
 * when bumping the vendored oxc-node in `.upstream-versions.json`.
 *
 * With `VPX_BUNDLE_CACHE=1`, `script-bundle-hooks.cjs` wraps the ESM hooks: a run with a
 * valid cached bundle loads the binding only when a module outside the bundle needs it.
 * It is a separate file, loaded only then, so other runs do not read its code.
 */

import type { LoadFnOutput, LoadHookSync, ResolveHookSync } from 'node:module';
import Module, { registerHooks, setSourceMapsSupport } from 'node:module';
import path from 'node:path';
import { isMainThread } from 'node:worker_threads';

import { addHook } from 'pirates';

import { installSignalAck } from '../../../oxc-node/packages/core/signal-ack.mjs';
import type { BundleCacheRun } from './script-bundle-hooks.ts';
import {
  canRegisterSyncHooks,
  flush,
  HELPER_MODULE_NAME,
  isCommonJsRequire,
  isSupportedNodeVersion,
  loadOxcNode,
  resolveWithOxcNode,
  routeHelperRequires,
  SUPPORTED_NODE_RANGE,
  threadScriptConfig,
  threadWatchSink,
  withFlush,
  withRequireAsyncModuleCode,
} from './script-hooks.ts';

if (!isSupportedNodeVersion(process.versions.node)) {
  process.stderr.write(
    `error: Running scripts with vpx requires Node.js ${SUPPORTED_NODE_RANGE} (current: ${process.version})\n`,
  );
  process.exit(1);
}

// Under `vpx --watch`, tell the watcher about each SIGINT the script handles, so a Ctrl+C
// the script already got from the terminal is not sent again. A no-op otherwise.
if (isMainThread) {
  installSignalAck();
}

// Under `vpx --watch`, report what Node.js does not: the tsconfig files oxc-node read
// and the paths a failed resolve looked for.
const watch = threadWatchSink();
// Pin this thread's config before its first transform or resolve. The ESM entry
// passes the same choice to the off-thread hooks on older Node.js releases.
const tsconfig = threadScriptConfig();
const cwd = process.cwd();
const bundleCache: BundleCacheRun | undefined =
  watch || process.env.VPX_BUNDLE_CACHE !== '1'
    ? undefined
    : (
        require(
          path.join(__dirname, 'script-bundle-hooks.cjs'),
        ) as typeof import('./script-bundle-hooks.ts')
      ).startBundleCache(__dirname);

type OxcNodeBinding = ReturnType<typeof loadOxcNode>;
let oxcNodeBinding: OxcNodeBinding | undefined;
/** While the binding loads, its own files are not the script's to transform. */
let loadingBinding = false;

/**
 * The native loader, set up on first use: before the first transform or resolve, as
 * `initializeLoader`, `setTransformDeps`, and `enableWatchReport` require. Loaded after the
 * version check so the native addon never loads on an unsupported Node.js.
 */
function oxcNode(): OxcNodeBinding {
  if (oxcNodeBinding === undefined) {
    let binding;
    loadingBinding = true;
    try {
      binding = loadOxcNode();
    } finally {
      loadingBinding = false;
    }
    if (watch) {
      binding.enableWatchReport();
    }
    binding.initializeLoader(tsconfig.path, tsconfig.source);
    // Dependencies run as published, except TypeScript, which Node.js refuses to strip
    // under `node_modules`.
    binding.setTransformDeps('typescript');
    oxcNodeBinding = binding;
  }
  return oxcNodeBinding;
}

if (!bundleCache?.warm) {
  oxcNode();
} else {
  bundleCache.reportBinding(() => oxcNodeBinding !== undefined);
}

const takeWatchDependencies = () => oxcNode().takeWatchDependencies();

setSourceMapsSupport(true, { nodeModules: true, generatedCode: true });
routeHelperRequires(Module as never);

const EXTENSIONS = ['.js', '.jsx', '.ts', '.tsx', '.mjs', '.mts', '.cjs', '.cts', '.es6', '.es'];
const NODE_MODULES = `${path.sep}node_modules${path.sep}`;
/** The dependencies `setTransformDeps('typescript')` transforms. */
const TYPESCRIPT_DEPENDENCY = /\.(?:[cm]?ts|tsx)$/;
const SOURCEMAP_PREFIX = '\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,';

let transformer: InstanceType<OxcNodeBinding['OxcTransformer']> | undefined;

addHook(
  (code, filename) => {
    let output;
    try {
      transformer ??= new (oxcNode().OxcTransformer)(cwd, HELPER_MODULE_NAME);
      output = transformer.transform(filename, code);
    } catch (error) {
      throw withRequireAsyncModuleCode(error);
    } finally {
      // A transform reads the tsconfig of the file it compiles.
      if (watch) {
        flush(watch, takeWatchDependencies);
      }
    }
    const source = output.source;
    const sourceMap = output.sourceMap;
    return sourceMap
      ? source + SOURCEMAP_PREFIX + Buffer.from(sourceMap, 'utf8').toString('base64')
      : source;
  },
  {
    exts: EXTENSIONS,
    // As oxc-node decides for `import`: dependencies run as published, except TypeScript.
    ignoreNodeModules: false,
    matcher: (filename) =>
      !loadingBinding && (!filename.includes(NODE_MODULES) || TYPESCRIPT_DEPENDENCY.test(filename)),
  },
);

// `require()` stays on Node.js' resolution without the binding, which itself loads
// through `require()` (see `resolveWithOxcNode`).
const resolve: ResolveHookSync = (specifier, context, nextResolve) =>
  isCommonJsRequire(context)
    ? nextResolve(specifier, context)
    : resolveWithOxcNode(oxcNode(), specifier, context, nextResolve);

const load: LoadHookSync = (url, context, nextLoad) => {
  if (isCommonJsRequire(context)) {
    return nextLoad(url, context);
  }
  const result = oxcNode().load(
    url,
    context as never,
    nextLoad as never,
    HELPER_MODULE_NAME,
  ) as LoadFnOutput & {
    responseURL?: string;
  };
  // oxc-node returns no source for a CommonJS file it leaves to the CommonJS loader,
  // which compiles it through `pirates` with an accurate source map; source that
  // another hook supplied is transformed and kept. A null source keeps `require()`
  // inside such a module working on every runtime (nodejs/node#62920).
  if (result.format === 'commonjs' && result.source == null) {
    // `@types/node` does not model the null source Node.js accepts for CommonJS.
    return {
      format: 'commonjs',
      source: null,
      responseURL: result.responseURL ?? url,
    } as unknown as LoadFnOutput;
  }
  return result;
};

if (canRegisterSyncHooks(process.versions.node)) {
  // Each hook reports what it read or looked for, even when it throws.
  registerHooks(
    watch
      ? {
          resolve: withFlush(resolve, watch, takeWatchDependencies),
          load: withFlush(load, watch, takeWatchDependencies),
        }
      : (bundleCache?.hooks({ resolve, load }) ?? { resolve, load }),
  );
}
