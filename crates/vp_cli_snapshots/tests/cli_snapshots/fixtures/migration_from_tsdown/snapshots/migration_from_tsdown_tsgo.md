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
  deps: { resolveDepSubpath: true, onlyBundle: false },
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

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
  deps: { resolveDepSubpath: true, onlyBundle: false },
});
```
