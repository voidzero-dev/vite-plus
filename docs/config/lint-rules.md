# Vite+ Oxlint Rules

Vite+ ships a small Oxlint JS plugin for rules that protect Vite+ project conventions. These rules are added by `vp lint --init`, `vp create`, and `vp migrate` through the `vite-plus/oxlint-plugin` entry in `lint.jsPlugins`.

## vite-plus/prefer-vite-plus-imports {#vite-plus-prefer-vite-plus-imports}

This rule rewrites direct Vite and Vitest imports to the Vite+ package entrypoints.

Examples:

```ts
import { defineConfig } from 'vite-plus';
import { test } from 'vite-plus/test';
```

Use this rule to keep application code on Vite+ entrypoints after migration.

## vite-plus/require-pnpm-vite-alias {#vite-plus-require-pnpm-vite-alias}

This rule protects pnpm monorepos where `pnpm-workspace.yaml` redirects `vite` through a catalog entry to `@voidzero-dev/vite-plus-core`.

In that setup, an application package that runs Vite through `vp dev`, `vp build`, or `vp preview` must keep a direct `vite` dependency, usually:

```json
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

Without the direct `vite` dependency, pnpm has no package-level consumer for the workspace override. Commands such as `vp why vite` can then report upstream `vite` from transitive dependencies instead of the Vite+ core alias, making the override look ineffective.

The rule only reports when all of these are true:

- Oxlint is checking the package's `vite.config.*` file
- the nearest package depends on `vite-plus`
- the package has an application script using `vp dev`, `vp build`, or `vp preview`
- the package is missing a direct `vite` dependency
- a parent `pnpm-workspace.yaml` redirects `vite` through a catalog entry to `@voidzero-dev/vite-plus-core`

Library and tooling packages that only use commands such as `vp pack`, `vp test`, or `vp check` are not considered application packages by this rule.
