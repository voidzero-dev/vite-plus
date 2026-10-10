/**
 * `--require` entry for `vpx <script>`, built to `dist/script-preload.cjs`.
 *
 * The CLIs pass `--require <dist/script-preload.cjs> --import <dist/script-register.js>`.
 * This preload runs first, before any user `--require` or `--import`, so a
 * `--require ./setup.ts` preload works too. It checks the Node.js version, installs the
 * CommonJS hook, and registers the in-thread ESM hooks. On Node.js releases before
 * 22.22.3 and 24.11.1, which lack `registerHooks()` fixes, `script-register.js`
 * registers off-thread ESM hooks instead; Node.js 22 cannot call `module.register()`
 * while a `--require` is loading. The `--import` also makes Node.js run the entry point
 * through the ESM loader, where these hooks decide each file's module format.
 *
 * It uses the oxc-node hooks compiled into the Vite+ native binding (exported as
 * `oxcNode`). Adapted from oxc-node's `packages/core/register.mjs`; keep them in sync
 * when bumping the vendored oxc-node in `.upstream-versions.json`.
 */

import type { LoadFnOutput, LoadHookSync, ResolveHookSync } from 'node:module';
import { createRequire, registerHooks, setSourceMapsSupport } from 'node:module';
import path from 'node:path';

import { addHook } from 'pirates';

import {
  canRegisterSyncHooks,
  isCommonJsRequire,
  isSupportedNodeVersion,
  resolveWithOxcNode,
  rewriteHelperRequires,
  SUPPORTED_NODE_RANGE,
} from './script-hooks.ts';

if (!isSupportedNodeVersion(process.versions.node)) {
  process.stderr.write(
    `error: Running scripts with vpx requires Node.js ${SUPPORTED_NODE_RANGE} (current: ${process.version})\n`,
  );
  process.exit(1);
}

// Loaded after the version check so the native addon never loads on an
// unsupported Node.js.
const { oxcNode } = createRequire(import.meta.url)(
  '../binding/index.cjs',
) as typeof import('../binding/index.js');

// `vpx --tsconfig` pins one config for every file. It must be set before the first
// transform or resolve; the off-thread hooks load the same native library, so they
// see it too. Without it, each file uses its nearest tsconfig.
oxcNode.setTsconfigPath(process.env.VP_SCRIPT_TSCONFIG);

setSourceMapsSupport(true, { nodeModules: true, generatedCode: true });

const EXTENSIONS = ['.js', '.jsx', '.ts', '.tsx', '.mjs', '.mts', '.cjs', '.cts', '.es6', '.es'];
const TYPESCRIPT_EXTENSIONS = new Set(['.ts', '.mts', '.cts', '.tsx']);
const NODE_MODULES = `${path.sep}node_modules${path.sep}`;
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
  {
    exts: EXTENSIONS,
    // Dependencies run as published, except TypeScript, which Node.js cannot run.
    ignoreNodeModules: false,
    matcher: (filename) =>
      !filename.includes(NODE_MODULES) || TYPESCRIPT_EXTENSIONS.has(path.extname(filename)),
  },
);

const resolve: ResolveHookSync = (specifier, context, nextResolve) =>
  resolveWithOxcNode(oxcNode, specifier, context, nextResolve);

const load: LoadHookSync = (url, context, nextLoad) => {
  if (isCommonJsRequire(context)) {
    return nextLoad(url, context);
  }
  const result = oxcNode.load(url, context as never, nextLoad as never) as LoadFnOutput & {
    responseURL?: string;
  };
  // Leave CommonJS to the CommonJS loader, which compiles it through `pirates` with
  // an accurate source map. A null source keeps `require()` inside such a module
  // working on every runtime (nodejs/node#62920).
  if (result.format === 'commonjs') {
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
