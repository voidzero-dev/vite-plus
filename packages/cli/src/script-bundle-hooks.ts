/**
 * The run side of the `vpx` bundle cache (see `script-bundle.ts`): the hooks that run a
 * cached bundle, and the recorder that feeds `script-bundle-build.ts`. Used by the
 * preload, which registers what `hooks()` returns around its own hooks.
 */

import { spawn, spawnSync } from 'node:child_process';
import Module, { type LoadFnOutput, type LoadHookSync, type ResolveHookSync } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isMainThread } from 'node:worker_threads';

import { ignoreInput, ignoreOutput } from '@voidzero-dev/vite-task-client';

import {
  acquireBuildLock,
  type BundleCacheContext,
  bundleCacheContext,
  type BundleManifest,
  type BundleTrace,
  BUNDLE_CACHE_VERSION,
  checkManifest,
  debug,
  debugEnabled,
  excludeModule,
  isBuiltinSpecifier,
  isPlainDependency,
  isPlainDependencyImport,
  parseResolveFrom,
  releaseBuildLock,
  TRACE,
  unsupportedReason,
  writeJsonAtomic,
} from './script-bundle.ts';
import { canRegisterSyncHooks, isCommonJsRequire } from './script-hooks.ts';

export interface LoaderHooks {
  resolve: ResolveHookSync;
  load: LoadHookSync;
}

export interface BundleCacheRun {
  /** The run uses a cached bundle, so the native binding can load when first needed. */
  warm: boolean;
  /** The hooks to register in place of the loader's own `full` hooks. */
  hooks(full: LoaderHooks): LoaderHooks;
  /** Under `VPX_BUNDLE_CACHE_DEBUG=1`, report at exit whether the binding loaded. */
  reportBinding(loaded: () => boolean): void;
}

/**
 * Start the bundle cache for this process, or return `undefined` when it does not apply:
 * without `VPX_BUNDLE_CACHE=1`, off the main thread, on a Node.js release without
 * in-thread hooks, under an option the cache does not support, or for a script outside
 * a project. `dist` is the directory of the preload and the other loader entries.
 */
export function startBundleCache(dist: string): BundleCacheRun | undefined {
  const env = process.env;
  if (env.VPX_BUNDLE_CACHE !== '1' || !isMainThread) {
    return undefined;
  }
  if (!canRegisterSyncHooks(process.versions.node)) {
    debug(`off (Node.js ${process.version} has no in-thread module hooks)`);
    return undefined;
  }
  const unsupported = unsupportedReason(env, process.execArgv, {
    preload: path.join(dist, 'script-preload.cjs'),
    register: path.join(dist, 'script-register.js'),
  });
  if (unsupported !== undefined) {
    debug(`off (${unsupported})`);
    return undefined;
  }
  const context = bundleCacheContext(process.argv[1], process.cwd(), env, process.execArgv);
  if (context === undefined) {
    debug('off (no node_modules directory above the script)');
    return undefined;
  }
  // Under `vp run`, the cache is this loader's own state, not a task input or output. The
  // checks below still stat every source file, which keeps those files task inputs.
  ignoreInput(context.root);
  ignoreOutput(context.root);
  const state = checkManifest(context);
  if (state.kind === 'hit') {
    debug(`hit (${state.manifest.modules?.length ?? 0} modules)`);
    return warmRun(context, state.manifest, dist);
  }
  if (state.kind === 'disabled') {
    debug(`off (${state.reason})`);
    return undefined;
  }
  debug(`miss (${state.reason}); recording`);
  return recordRun(context, dist);
}

/** Node.js' own compile cache for the bundle, unless the user configured one. */
function enableCompileCache(root: string): void {
  const env = process.env;
  if (env.NODE_COMPILE_CACHE !== undefined || env.NODE_DISABLE_COMPILE_CACHE !== undefined) {
    return;
  }
  Module.enableCompileCache?.(path.join(root, 'v8'));
}

/**
 * Run the bundle: the main entry resolves to the entry chunk, and the chunks and the
 * loader's own files go straight to Node.js. Everything else goes to the loader's hooks,
 * which load the native binding on first use. The bundle never loads a module from its
 * own file, so a load of a bundled module's URL means a second instance of it, for
 * example from a computed `import()` the recorded run did not make. The run reports it
 * once and drops the bundle, and the next build leaves the module out.
 */
function warmRun(
  context: BundleCacheContext,
  manifest: BundleManifest,
  dist: string,
): BundleCacheRun {
  const outURL = `${pathToFileURL(path.join(context.dir, manifest.out!)).href}/`;
  const entryChunkURL = outURL + manifest.entry!;
  const distURL = `${pathToFileURL(dist).href}/`;
  const modules = manifest.modules ?? [];
  const bundled = new Set(modules);
  enableCompileCache(context.root);

  let reported = false;
  const checkDuplicate = (url: string) => {
    if (reported || !bundled.has(url)) {
      return;
    }
    reported = true;
    excludeModule(context.dir, url);
    process.stderr.write(
      `vpx: warning: ${fileURLToPath(url)} is in the bundle cache but also loaded from its own file, so it has two instances in this run; the next run rebuilds the cache without it\n`,
    );
  };

  return {
    warm: true,
    reportBinding(loaded) {
      if (debugEnabled()) {
        process.once('exit', () => {
          debug(`the native loader ${loaded() ? 'loaded' : 'did not load'}`);
        });
      }
    },
    hooks(full) {
      const resolve: ResolveHookSync = (specifier, hookContext, nextResolve) => {
        const parentURL = hookContext.parentURL;
        if (!isCommonJsRequire(hookContext)) {
          if (parentURL === undefined && specifier === context.entryURL) {
            return { url: entryChunkURL, format: 'module', shortCircuit: true };
          }
          if (isBuiltinSpecifier(specifier)) {
            // The loader short-circuits built-ins to Node.js too.
            return nextResolve(specifier, hookContext);
          }
          if (parentURL?.startsWith(outURL)) {
            const from = parseResolveFrom(specifier);
            if (from !== undefined && modules[from[0]] !== undefined) {
              // A computed `import()` or `import.meta.resolve()` in a bundled module:
              // resolved from that module's own URL, as without the bundle.
              return full.resolve(
                from[1],
                { ...hookContext, parentURL: modules[from[0]] },
                nextResolve,
              );
            }
            // Other chunks, and dependencies resolved at build time to the `file:` URL that
            // Node.js resolves the same way.
            if (specifier.startsWith('./') || isPlainDependency(specifier)) {
              return nextResolve(specifier, hookContext);
            }
          } else if (parentURL?.startsWith(distURL) || isOwnRegister(specifier, distURL)) {
            return nextResolve(specifier, hookContext);
          } else if (parentURL !== undefined && isPlainDependencyImport(specifier, parentURL)) {
            // A dependency outside the bundle importing its own files.
            return nextResolve(specifier, hookContext);
          }
        }
        return full.resolve(specifier, hookContext, nextResolve);
      };
      const load: LoadHookSync = (url, hookContext, nextLoad) => {
        if (url.startsWith('node:') || url.startsWith(outURL) || url.startsWith(distURL)) {
          return nextLoad(url, hookContext);
        }
        checkDuplicate(url);
        if (!isCommonJsRequire(hookContext) && isPlainDependency(url)) {
          // What the loader does for these: Node.js' answer, and no source for CommonJS,
          // which the CommonJS loader reads itself (nodejs/node#62920).
          const result = nextLoad(url, hookContext);
          return result.format === 'commonjs'
            ? ({ format: 'commonjs', source: null, responseURL: url } as unknown as LoadFnOutput)
            : result;
        }
        return full.load(url, hookContext, nextLoad);
      };
      return { resolve, load };
    },
  };
}

/** The `--import` entry, whichever spelling of its URL the CLI passed. */
function isOwnRegister(specifier: string, distURL: string): boolean {
  if (!specifier.startsWith('file:') || !specifier.endsWith('/script-register.js')) {
    return false;
  }
  try {
    return pathToFileURL(path.dirname(fileURLToPath(specifier))).href + '/' === distURL;
  } catch {
    return false;
  }
}

/**
 * Run the script through the loader's hooks and record how every import resolved and
 * every module loaded. At exit, take the build lock, write the trace, and start the
 * builder: detached, so the script's exit does not wait for it, or in this process under
 * `VPX_BUNDLE_CACHE_DEBUG=1`. The lock covers only the build, which is short; the run
 * itself can last any time. A run that finds the lock taken leaves the build to its holder.
 */
function recordRun(context: BundleCacheContext, dist: string): BundleCacheRun {
  const env = { ...process.env };
  const conditions: string[][] = [];
  const conditionIndex = new Map<string, number>();
  const resolves: BundleTrace['resolves'] = [];
  const loads: BundleTrace['loads'] = [];

  process.once('exit', () => {
    if (!acquireBuildLock(context.dir)) {
      debug('another run is building the bundle', env);
      return;
    }
    try {
      const trace: BundleTrace = {
        version: BUNDLE_CACHE_VERSION,
        key: context.key,
        entry: context.entryURL,
        cwd: context.cwd,
        conditions,
        resolves,
        loads,
      };
      writeJsonAtomic(path.join(context.dir, TRACE), trace);
      const args = [path.join(dist, 'script-bundle-build.js'), context.dir];
      if (debugEnabled(env)) {
        spawnSync(process.execPath, args, {
          cwd: context.cwd,
          env,
          stdio: ['ignore', 'ignore', 'inherit'],
        });
      } else {
        spawn(process.execPath, args, {
          cwd: context.cwd,
          env,
          stdio: 'ignore',
          detached: true,
          windowsHide: true,
        }).unref();
      }
    } catch (error) {
      releaseBuildLock(context.dir);
      debug(`could not start the build: ${(error as Error).message}`, env);
    }
  });

  return {
    warm: false,
    reportBinding() {},
    hooks(full) {
      return {
        resolve(specifier, hookContext, nextResolve) {
          const result = full.resolve(specifier, hookContext, nextResolve);
          const list = hookContext.conditions ?? [];
          const key = list.join('\0');
          let index = conditionIndex.get(key);
          if (index === undefined) {
            index = conditions.push([...list]) - 1;
            conditionIndex.set(key, index);
          }
          resolves.push([
            hookContext.parentURL ?? null,
            specifier,
            isCommonJsRequire(hookContext) ? 'require' : 'import',
            index,
            result.url,
            result.format ?? null,
          ]);
          return result;
        },
        load(url, hookContext, nextLoad) {
          const result = full.load(url, hookContext, nextLoad);
          loads.push([url, result.format ?? null]);
          return result;
        },
      };
    },
  };
}
