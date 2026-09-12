import { VITEST_VERSION, VITEST_WEBDRIVERIO_RANGE } from './constants.ts';

/** Official packages that share the bundled runner's release version. */
export const VITEST_EXACT_VERSION_PACKAGES: ReadonlySet<string> = new Set([
  '@vitest/browser',
  '@vitest/browser-playwright',
  '@vitest/browser-preview',
  '@vitest/coverage-v8',
  '@vitest/coverage-istanbul',
  '@vitest/mocker',
  '@vitest/pretty-format',
  '@vitest/snapshot',
  '@vitest/spy',
  '@vitest/ui',
  '@vitest/utils',
  '@vitest/web-worker',
]);

/** Optional peers fall back to the project; unrelated @vitest packages do not redirect. */
export const VITEST_RESOLVER_PACKAGES: ReadonlySet<string> = new Set([
  'vitest',
  ...VITEST_EXACT_VERSION_PACKAGES,
  '@vitest/browser-webdriverio',
]);

export function isAlignableVitestEcosystemPackage(name: string): boolean {
  return VITEST_EXACT_VERSION_PACKAGES.has(name) || name === '@vitest/browser-webdriverio';
}

export function vitestEcosystemVersion(name: string): string {
  return name === '@vitest/browser-webdriverio' ? VITEST_WEBDRIVERIO_RANGE : VITEST_VERSION;
}
