import { VITEST_VERSION, VITEST_WEBDRIVERIO_VERSION } from './constants.ts';

const VITEST_ALIGN_EXCLUDED = new Set([
  '@vitest/eslint-plugin',
  // Vitest v5 no longer publishes this package. Keep a user declaration in
  // place so the migration can report the unsupported internal API instead of
  // rewriting it to a version that does not exist.
  '@vitest/runner',
  // Deprecated at 0.33.0 and replaced by @vitest/coverage-v8. It does not
  // publish versions on Vitest's current release line.
  '@vitest/coverage-c8',
]);

export function isAlignableVitestEcosystemPackage(name: string): boolean {
  return name.startsWith('@vitest/') && !VITEST_ALIGN_EXCLUDED.has(name);
}

export function getVitestEcosystemVersion(name: string): string {
  return name === '@vitest/browser-webdriverio' ? VITEST_WEBDRIVERIO_VERSION : VITEST_VERSION;
}
