// The Oxlint JS-plugin authoring API, re-exported from the copy of
// `@oxlint/plugins` that ships with Vite+.
//
// Oxlint used to expose `defineRule`/`definePlugin` from its main entry; they
// now live in `@oxlint/plugins`, and the plugin API is versioned against the
// linter that loads the plugin. Since `vp lint` runs the bundled Oxlint, a
// project that declares its own `@oxlint/plugins` has to keep that pin in sync
// with whatever Vite+ bundles. Importing from here removes that pin: the API is
// always the one the bundled linter understands, and it resolves from any
// package that already has `vite-plus` installed (`@oxlint/plugins` is a
// transitive dependency and is therefore NOT resolvable from a user's plugin
// file under pnpm's strict layout).
//
// `vp migrate` rewrites legacy `oxlint` / `@oxlint/plugins` plugin-API imports
// to this specifier, and the `vite-plus/prefer-vite-plus-imports` lint rule
// enforces it. See `crates/vp_migration/src/import_rewriter.rs` and
// `packages/cli/src/oxlint-plugin.ts`; the two mappings must stay in sync.

export { definePlugin, defineRule, eslintCompatPlugin } from '@oxlint/plugins';
export type * from '@oxlint/plugins';
