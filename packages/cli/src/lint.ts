// Keep standalone oxlint.config.ts files resolvable after migration removes the
// direct `oxlint` dependency. Root Vite+ configs should still import the unified
// `defineConfig()` from `vite-plus`.
export { defineConfig } from 'oxlint';
export type * from 'oxlint';
