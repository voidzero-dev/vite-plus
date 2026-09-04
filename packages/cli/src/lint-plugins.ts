// The Oxlint JS-plugin authoring API, re-exported from the copy of
// `@oxlint/plugins` that ships with Vite+.
//
// Oxlint used to expose `defineRule` and `definePlugin` from its main entry.
// They now live in `@oxlint/plugins`. The plugin API is versioned against the
// linter that loads the plugin, and `vp lint` runs the bundled Oxlint. So a
// project that declares its own `@oxlint/plugins` must keep that pin in step
// with whatever Vite+ bundles.
//
// An import from here removes that pin. The API is always the one the bundled
// linter understands. It also resolves from any package that already has
// `vite-plus` installed. A direct `@oxlint/plugins` import does not:
// `@oxlint/plugins` is a transitive dependency, which pnpm's strict layout
// hides from a user's plugin file.
//
// `vp migrate` rewrites legacy `oxlint` and `@oxlint/plugins` authoring imports
// to this specifier. The `vite-plus/prefer-vite-plus-imports` lint rule
// enforces it. See `crates/vp_migration/src/import_rewriter.rs` and
// `packages/cli/src/oxlint-plugin.ts`. The two mappings must stay in sync.

export { definePlugin, defineRule, eslintCompatPlugin } from '@oxlint/plugins';
export type * from '@oxlint/plugins';
