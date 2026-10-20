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
 */

import type { LoadFnOutput, LoadHookSync, ResolveHookSync } from 'node:module';
import Module, { registerHooks, setSourceMapsSupport } from 'node:module';
import path from 'node:path';

import { addHook } from 'pirates';

import {
  canRegisterSyncHooks,
  HELPER_MODULE_NAME,
  isCommonJsRequire,
  isSupportedNodeVersion,
  loadOxcNode,
  resolveWithOxcNode,
  routeHelperRequires,
  SUPPORTED_NODE_RANGE,
  withRequireAsyncModuleCode,
} from './script-hooks.ts';

if (!isSupportedNodeVersion(process.versions.node)) {
  process.stderr.write(
    `error: Running scripts with vpx requires Node.js ${SUPPORTED_NODE_RANGE} (current: ${process.version})\n`,
  );
  process.exit(1);
}

// Loaded after the version check so the native addon never loads on an
// unsupported Node.js.
const oxcNode = loadOxcNode();

// `vpx --tsconfig` pins one config for every file. It must be set before the first
// transform or resolve; the off-thread hooks load the same native library, so they
// see it too. Without it, each file uses its nearest tsconfig.
oxcNode.setTsconfigPath(process.env.VP_SCRIPT_TSCONFIG);
// Dependencies run as published, except TypeScript, which Node.js refuses to strip
// under `node_modules`. Set the same way, before the first transform or resolve.
oxcNode.setTransformDeps('typescript');

setSourceMapsSupport(true, { nodeModules: true, generatedCode: true });
routeHelperRequires(Module as never);

const EXTENSIONS = ['.js', '.jsx', '.ts', '.tsx', '.mjs', '.mts', '.cjs', '.cts', '.es6', '.es'];
const NODE_MODULES = `${path.sep}node_modules${path.sep}`;
const SOURCEMAP_PREFIX = '\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,';

const transformer = new oxcNode.OxcTransformer(process.cwd(), HELPER_MODULE_NAME);

addHook(
  (code, filename) => {
    let output;
    try {
      output = transformer.transform(filename, code);
    } catch (error) {
      throw withRequireAsyncModuleCode(error);
    }
    const source = output.source;
    const sourceMap = output.sourceMap;
    return sourceMap
      ? source + SOURCEMAP_PREFIX + Buffer.from(sourceMap, 'utf8').toString('base64')
      : source;
  },
  {
    exts: EXTENSIONS,
    // oxc-node decides which dependencies to transform, as for `import`.
    ignoreNodeModules: false,
    matcher: (filename) => !filename.includes(NODE_MODULES) || oxcNode.transformsDep(filename),
  },
);

const resolve: ResolveHookSync = (specifier, context, nextResolve) =>
  resolveWithOxcNode(oxcNode, specifier, context, nextResolve);

const load: LoadHookSync = (url, context, nextLoad) => {
  if (isCommonJsRequire(context)) {
    return nextLoad(url, context);
  }
  const result = oxcNode.load(
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
  registerHooks({ resolve, load });
}
