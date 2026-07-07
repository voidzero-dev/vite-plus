import { createRequire } from 'node:module';

import cliPkg from '../../package.json' with { type: 'json' };

export const VITE_PLUS_NAME = 'vite-plus';
export const VITE_PLUS_VERSION = process.env.VP_VERSION || cliPkg.version;

export const VITEST_VERSION = '4.1.10';

export const VITE_PLUS_OVERRIDE_PACKAGES: Record<string, string> = process.env.VP_OVERRIDE_PACKAGES
  ? JSON.parse(process.env.VP_OVERRIDE_PACKAGES)
  : {
      vite: `npm:@voidzero-dev/vite-plus-core@${VITE_PLUS_VERSION}`,
      // Pin `vitest` only. The `@vitest/*` family (expect, runner, snapshot, spy,
      // utils, mocker, pretty-format) are EXACT (`4.1.9`) dependencies of `vitest`
      // itself, so a single `vitest` override cascades one consistent version to
      // the whole tree — overriding the indirect deps individually is redundant.
      // Coverage providers (`@vitest/coverage-v8` / `-istanbul`) are vitest PEER
      // deps the project installs and versions itself — vite-plus never adds,
      // pins, or overrides them. The runtime guard in `define-config.ts` only
      // fail-fasts when an installed provider's version skews from the bundled
      // vitest (Vitest would otherwise silently run mixed versions).
      vitest: VITEST_VERSION,
    };

/**
 * Package-name patterns the migrator exempts from a package manager's
 * "minimum release age" gate (pnpm `minimumReleaseAgeExclude` / Yarn
 * `npmPreapprovedPackages`).
 *
 * Vite+ pins `vitest` to an exact, sometimes freshly published version, and the
 * in-tree `@vitest/*` siblings install transitively at that same version, so an
 * age gate would otherwise quarantine the Vite+-managed family and break
 * `vp install`. The `@vitest/*` glob also covers the optional `@vitest/browser-*`
 * peers the migrator pins for browser projects. This does NOT pin or manage any
 * package — it only lets the chosen versions through the user's gate, including
 * the `@vitest/coverage-*` version the coverage guard asks the user to align to
 * the bundled vitest.
 */
export const VITEST_AGE_GATE_EXEMPT_PACKAGES = ['vitest', '@vitest/*'] as const;

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

// Env var set while `vite.config.ts` is loaded only to read a config block, not
// to run the Vite pipeline. `lazyPlugins` skips the user's plugin factory while
// it is `'1'`. Single source of truth shared by `withConfigMetadataResolution`
// (in-process) and the oxlint/oxfmt resolvers + bins (which load the config in
// a subprocess). Keep the `bin/oxlint`/`bin/oxfmt` literals in sync with this.
export const CONFIG_METADATA_ENV = 'VP_RESOLVING_CONFIG_METADATA';
