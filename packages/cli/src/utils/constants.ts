import { createRequire } from 'node:module';

import cliPkg from '../../package.json' with { type: 'json' };

export const VITE_PLUS_NAME = 'vite-plus';

// Range derived from the running CLI's own version (cli/core/test are
// published in lockstep). A real range instead of the `latest` dist-tag keeps
// `vp update vite-plus` effective: package managers re-resolve ranges but
// leave dist-tag specs untouched in the lockfile.
export const VITE_PLUS_VERSION_RANGE = `^${cliPkg.version}`;

export const VITE_PLUS_VERSION = process.env.VP_VERSION || VITE_PLUS_VERSION_RANGE;

export const VITE_PLUS_OVERRIDE_PACKAGES: Record<string, string> = process.env.VP_OVERRIDE_PACKAGES
  ? JSON.parse(process.env.VP_OVERRIDE_PACKAGES)
  : {
      vite: `npm:@voidzero-dev/vite-plus-core@${VITE_PLUS_VERSION_RANGE}`,
      vitest: `npm:@voidzero-dev/vite-plus-test@${VITE_PLUS_VERSION_RANGE}`,
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
