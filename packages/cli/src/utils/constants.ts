import { createRequire } from 'node:module';

export const VITE_PLUS_NAME = 'vite-plus';
export const VITE_PLUS_VERSION = process.env.VP_VERSION || 'latest';

export const VITE_PLUS_OVERRIDE_PACKAGES: Record<string, string> = process.env.VP_OVERRIDE_PACKAGES
  ? JSON.parse(process.env.VP_OVERRIDE_PACKAGES)
  : {
      vite: 'npm:@voidzero-dev/vite-plus-core@latest',
      vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
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
