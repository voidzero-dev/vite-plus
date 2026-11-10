# migration_pack_tsdown_defaults_legacy

An older Vite+ installation without a toolchain manifest receives compatibility defaults, while explicit values and the second migration remain unchanged.

## `vpt replace-file-content pnpm-workspace.yaml @1.0.0 @0.2.0`


## `vpt replace-file-content pnpm-workspace.yaml 'vite-plus: 1.0.0' 'vite-plus: 0.2.0'`


## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.2.0","bundledVersions":{"vite":"8.0.0"}}'`


## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.0"}'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite       8.0.0 → <version>
• 2 files had imports rewritten
• Package manager settings configured
```

## `vpt print-file apps/server-v2/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: { deps: {
      // tsdown <0.23 compatibility: resolve external dependency subpaths.
      // Remove to preserve subpath imports as written (the new default).
      // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
      resolveDepSubpath: true },
    entry: ['src/main.ts'],
    format: ['esm'],
  },
});
```

## `vpt print-file packages/library/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: [
    { deps: {
        // tsdown <0.23 compatibility: resolve external dependency subpaths.
        // Remove to preserve subpath imports as written (the new default).
        // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
        resolveDepSubpath: true },
      entry: ['src/index.ts'],
      attw: {
        // tsdown <0.23 compatibility: keep all declaration resolution checks.
        // Remove to use the new 'esm-only' profile.
        // https://tsdown.dev/options/lint#profiles
        profile: 'strict' },
    },
    {
      entry: ['src/index.ts'],
      deps: {
        // tsdown <0.23 compatibility: resolve external dependency subpaths.
        // Remove to preserve subpath imports as written (the new default).
        // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
        resolveDepSubpath: true,},
      attw: {
        // tsdown <0.23 compatibility: keep all declaration resolution checks.
        // Remove to use the new 'esm-only' profile.
        // https://tsdown.dev/options/lint#profiles
        profile: 'strict', enabled: true },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: false },
      attw: { profile: 'esm-only' },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: true },
      attw: { profile: 'strict' },
    },
  ],
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file apps/server-v2/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: { deps: {
      // tsdown <0.23 compatibility: resolve external dependency subpaths.
      // Remove to preserve subpath imports as written (the new default).
      // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
      resolveDepSubpath: true },
    entry: ['src/main.ts'],
    format: ['esm'],
  },
});
```

## `vpt print-file packages/library/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: [
    { deps: {
        // tsdown <0.23 compatibility: resolve external dependency subpaths.
        // Remove to preserve subpath imports as written (the new default).
        // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
        resolveDepSubpath: true },
      entry: ['src/index.ts'],
      attw: {
        // tsdown <0.23 compatibility: keep all declaration resolution checks.
        // Remove to use the new 'esm-only' profile.
        // https://tsdown.dev/options/lint#profiles
        profile: 'strict' },
    },
    {
      entry: ['src/index.ts'],
      deps: {
        // tsdown <0.23 compatibility: resolve external dependency subpaths.
        // Remove to preserve subpath imports as written (the new default).
        // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
        resolveDepSubpath: true,},
      attw: {
        // tsdown <0.23 compatibility: keep all declaration resolution checks.
        // Remove to use the new 'esm-only' profile.
        // https://tsdown.dev/options/lint#profiles
        profile: 'strict', enabled: true },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: false },
      attw: { profile: 'esm-only' },
    },
    {
      entry: ['src/index.ts'],
      deps: { resolveDepSubpath: true },
      attw: { profile: 'strict' },
    },
  ],
});
```
