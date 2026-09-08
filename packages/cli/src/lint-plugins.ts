// Use the plugin API that matches the bundled linter. This entry also makes
// the API accessible through a direct vite-plus dependency under strict pnpm.
// The migrator and prefer-vite-plus-imports rule both target this entry.

export { definePlugin, defineRule, eslintCompatPlugin } from '@oxlint/plugins';
export type * from '@oxlint/plugins';
