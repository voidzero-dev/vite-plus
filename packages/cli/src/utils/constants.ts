import { createRequire } from 'node:module';

export const VITE_PLUS_NAME = 'vite-plus';
export const VITE_PLUS_VERSION = process.env.VP_VERSION || 'latest';

export const VITEST_VERSION = '4.1.7';

export const VITE_PLUS_OVERRIDE_PACKAGES: Record<string, string> = process.env.VP_OVERRIDE_PACKAGES
  ? JSON.parse(process.env.VP_OVERRIDE_PACKAGES)
  : {
      vite: 'npm:@voidzero-dev/vite-plus-core@latest',
      vitest: VITEST_VERSION,
      '@vitest/expect': VITEST_VERSION,
      '@vitest/runner': VITEST_VERSION,
      '@vitest/snapshot': VITEST_VERSION,
      '@vitest/spy': VITEST_VERSION,
      '@vitest/utils': VITEST_VERSION,
      '@vitest/mocker': VITEST_VERSION,
      '@vitest/pretty-format': VITEST_VERSION,
      '@vitest/coverage-v8': VITEST_VERSION,
      '@vitest/coverage-istanbul': VITEST_VERSION,
    };

/**
 * When VP_FORCE_MIGRATE is set, force full dependency rewriting
 * even for projects already using vite-plus. Used by ecosystem CI to
 * override dependencies with locally built tgz packages.
 */
export function isForceOverrideMode(): boolean {
  return process.env.VP_FORCE_MIGRATE === '1';
}

const require = createRequire(import.meta.url);

export function resolve(path: string) {
  return require.resolve(path, {
    paths: [process.cwd(), import.meta.dirname],
  });
}

/**
 * Like {@link resolve}, but prefers the copy shipped with the CLI
 * (`import.meta.dirname`) over the project's (`process.cwd()`).
 *
 * Use this for runtime modules that MUST match what `vite-plus/test*`
 * imports resolve to — chiefly the Vitest runner binary. The `vite-plus/test`
 * shims `export * from 'vitest'`, which Node resolves to vite-plus's own
 * bundled (pinned) Vitest. If `vp test` instead spawned a project-local
 * Vitest (a different physical copy/version), the runner and the imported
 * `vi`/`expect`/runner internals would come from two distinct Vitest
 * modules — a classic source of Vitest internal-state and mock-hoisting
 * mismatches. `process.cwd()` stays as a fallback for layouts where the
 * bundled copy is somehow unreachable, so this is never worse than {@link resolve}.
 */
export function resolveBundled(path: string) {
  return require.resolve(path, {
    paths: [import.meta.dirname, process.cwd()],
  });
}

export const BASEURL_TSCONFIG_WARNING =
  'Skipped typeAware/typeCheck: a tsconfig file contains baseUrl which is not yet supported by the oxlint type checker.\n' +
  '  Run `vp dlx @andrewbranch/ts5to6 --fixBaseUrl <tsconfig path>` to remove baseUrl from your tsconfig.';

export const BASEURL_TSCONFIG_FIX_PACKAGE = '@andrewbranch/ts5to6';
export const BASEURL_TSCONFIG_FIX_FLAG = '--fixBaseUrl';
export const BASEURL_TSCONFIG_FIX_DEFAULT_TARGET = '.';

export function createBaseUrlTsconfigFixArgs(target = BASEURL_TSCONFIG_FIX_DEFAULT_TARGET) {
  return [BASEURL_TSCONFIG_FIX_FLAG, target] as const;
}

export const DEFAULT_ENVS = {
  // Provide Node.js runtime information for oxfmt's telemetry/compatibility
  JS_RUNTIME_VERSION: process.versions.node,
  JS_RUNTIME_NAME: process.release.name,
  // Indicate that vite-plus is the package manager
  NODE_PACKAGE_MANAGER: 'vite-plus',
} as const;
