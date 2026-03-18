// Regression test for DTS warning fixes (PR #993).
//
// Importing UserConfig from vite-plus triggers DTS bundling of the full
// vite type chain. This test verifies that no postcss/lightningcss
// IMPORT_IS_UNDEFINED or vitest MISSING_EXPORT warnings are emitted.
//
// NOTE(kazupon): The snap.txt includes warnings from sass, esbuild, and immutable.
// These are snap-test environment artifacts - in the monorepo, these
// optional dependencies exist in node_modules/.pnpm/ and the DTS bundler
// resolves them, finding upstream type issues. In a normal user environment
// these packages are not installed, so the DTS bundler skips them and
// no warnings appear.
import type { UserConfig } from 'vite-plus';

export function defineConfig(config: UserConfig): UserConfig {
  return config;
}
