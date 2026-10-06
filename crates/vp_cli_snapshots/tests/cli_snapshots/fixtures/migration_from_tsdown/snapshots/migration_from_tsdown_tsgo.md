# migration_from_tsdown_tsgo

Convert dts.tsgo to dts.generator and preserve the result on a second migration.

## `vpt replace-file-content tsdown.config.ts 'dts: true' 'dts: { tsgo: true }'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied, 1 file had imports rewritten
→ Manual follow-up:
  - Please manually merge tsdown.config.ts into vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt print-file tsdown.config.ts`

check dts.generator replaces dts.tsgo

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
  outDir: 'dist',
  format: ['esm', 'cjs'],
  dts: { generator: 'tsgo' },
  unbundle: true,
  copy: 'public',
  deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true, onlyBundle: false },
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file tsdown.config.ts`

check the migrated config is unchanged

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
  outDir: 'dist',
  format: ['esm', 'cjs'],
  dts: { generator: 'tsgo' },
  unbundle: true,
  copy: 'public',
  deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true, onlyBundle: false },
});
```
