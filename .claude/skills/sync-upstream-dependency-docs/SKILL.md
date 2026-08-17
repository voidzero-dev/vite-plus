---
name: sync-upstream-dependency-docs
description: Sync exact upstream dependency versions embedded in Vite+ documentation after the automated dependency upgrade. Use when bundled Vite, Vitest, Oxlint, Oxfmt, tsdown, Rolldown, or related package versions change.
allowed-tools: Read, Grep, Glob, Edit, Bash
---

# Sync upstream dependency docs

1. Read `$UPGRADE_DEPS_META_DIR/versions.json` and consider only entries whose `old`
   and `new` values differ.
2. Search `README.md`, `packages/*/README.md`, and `docs/**/*.md` for each changed
   package name and its old exact version. Do not treat changelogs, RFC examples,
   snapshots, or broad ranges such as `vitest@4.x` as current-version references.
3. Update references that promise to match Vite+'s currently bundled version. In
   particular, keep every exact Vitest pin in the manual-migration examples in sync,
   including `docs/guide/migrate.md`, `README.md`, and `packages/cli/README.md` when
   those examples are present.
4. Preserve the surrounding wording and formatting. Do not rewrite examples whose
   version is intentionally historical or illustrative.
5. Re-run the searches for the changed packages and inspect the focused diff. No
   stale exact version may remain in documentation that describes the current bundle.
