const VITEST_ALIGN_EXCLUDED = new Set([
  '@vitest/eslint-plugin',
  // Deprecated at 0.33.0 and replaced by @vitest/coverage-v8. It does not
  // publish versions on Vitest's current release line.
  '@vitest/coverage-c8',
]);

export function isAlignableVitestEcosystemPackage(name: string): boolean {
  return name.startsWith('@vitest/') && !VITEST_ALIGN_EXCLUDED.has(name);
}
