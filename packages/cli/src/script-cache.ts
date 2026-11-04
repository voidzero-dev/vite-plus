/** Optional persistent caches for the script loader, shared by its CJS and ESM entries. */
import Module from 'node:module';
import path from 'node:path';

import { ignoreInput, ignoreOutput } from '@voidzero-dev/vite-task-client';

export interface ScriptCachePolicy {
  transform: boolean;
  compile: boolean;
  runtimeVersion: string;
  /** The native binding resolves this owned directory through VpDirs. */
  root?: string;
}

interface CacheBinding {
  initializeCache(
    enabled: boolean,
    runtimeVersion: string,
    root?: string,
  ): string | null | undefined;
}

const CACHE_POLICY = Symbol.for('vite-plus:script-cache-policy');

/** Opt in explicitly while the persistent cache is a performance prototype. */
export function scriptCachePolicy(
  env: NodeJS.ProcessEnv = process.env,
  execArgv: readonly string[] = process.execArgv,
  runtimeVersion = process.versions.node,
): ScriptCachePolicy {
  const mode = env.VPX_DISABLE_CACHE === '1' ? undefined : env.VPX_CACHE;
  // Cached V8 code can make precise coverage less accurate. Keep the user's own
  // compile-cache selection intact, but do not enable a managed one for coverage.
  const coverage =
    env.NODE_V8_COVERAGE !== undefined ||
    execArgv.some((arg) => /^--experimental[-_]test[-_]coverage(?:=|$)/.test(arg)) ||
    /(?:^|[\s"])--experimental[-_]test[-_]coverage(?:[=\s"]|$)/.test(env.NODE_OPTIONS ?? '');
  return {
    transform: mode === '1' || mode === 'transform',
    compile:
      (mode === '1' || mode === 'compile') &&
      env.NODE_DISABLE_COMPILE_CACHE !== '1' &&
      env.NODE_COMPILE_CACHE === undefined &&
      !coverage,
    runtimeVersion,
  };
}

/**
 * Initialize this thread before its first transform. Off-thread hooks receive the
 * frozen policy and root, rather than reading their stale environment snapshot.
 */
export function initializeScriptCache(
  binding: CacheBinding,
  frozen?: ScriptCachePolicy,
): ScriptCachePolicy {
  const policy = { ...(frozen ?? scriptCachePolicy()) };
  // An earlier preload can enable its own cache without NODE_COMPILE_CACHE. Do not
  // change or exclude that arbitrary directory either.
  if (!frozen && policy.compile && Module.getCompileCacheDir?.()) {
    policy.compile = false;
  }
  if (policy.transform || policy.compile) {
    const root = binding.initializeCache(policy.transform, policy.runtimeVersion, policy.root);
    if (root) {
      policy.root = root;
      // Only exclude the binding's dedicated owned root. In particular, never
      // exclude NODE_COMPILE_CACHE: the user can set it to their workspace root.
      // The backend creates no files until a transform is stored.
      ignoreInput(root);
      ignoreOutput(root);
      if (policy.compile) {
        const result = Module.enableCompileCache?.(path.join(root, 'node'));
        const status = Module.constants?.compileCacheStatus;
        policy.compile =
          result !== undefined &&
          (result.status === status?.ENABLED || result.status === status?.ALREADY_ENABLED);
      }
    } else {
      // A rejected/unavailable cache root stays disabled in the hooks thread too.
      policy.transform = false;
      policy.compile = false;
    }
  }
  (globalThis as { [CACHE_POLICY]?: ScriptCachePolicy })[CACHE_POLICY] = policy;
  return policy;
}

/** The preload and register entries are separate bundles in the same thread. */
export function freezeScriptCache(): ScriptCachePolicy {
  return (
    (globalThis as { [CACHE_POLICY]?: ScriptCachePolicy })[CACHE_POLICY] ?? {
      transform: false,
      compile: false,
      runtimeVersion: process.versions.node,
    }
  );
}
