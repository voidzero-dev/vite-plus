# migration_pack_tsdown_023_concise_methods

Migrate a standalone concise arrow and method options, then check unbundled files and copied assets.

## `vpt cp concise.config.txt tsdown.config.ts`


## `vpt write-file vite.config.ts 'import packConfig from '\''./tsdown.config.js'\'';
export default { pack: packConfig({}) };
'`


## `vpt cp concise-entry.txt src/index.ts`


## `vpt cp helper.txt src/helper.ts`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite             → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig(() => ({ deps: { resolveDepSubpath: true },
  entry: 'src/index.ts',
  unbundle: true,
  dts: { generator: 'oxc' },
  outExtensions() { return { js: '.custom.js' }; },
  copy() { return ['assets']; },
}));
```

## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
ℹ dist/index.d.custom.ts   <size> kB │ gzip: <size> kB
ℹ dist/index.custom.js     <size> kB │ gzip: <size> kB
ℹ dist/helper.d.custom.ts  <size> kB │ gzip: <size> kB
ℹ dist/helper.custom.js    <size> kB │ gzip: <size> kB
ℹ 4 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt stat-file dist/index.custom.js --assert file`

```
dist/index.custom.js: file
```

## `vpt stat-file dist/helper.custom.js --assert file`

```
dist/helper.custom.js: file
```

## `vpt print-file dist/assets/asset.txt`

```
method copy asset
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig(() => ({ deps: { resolveDepSubpath: true },
  entry: 'src/index.ts',
  unbundle: true,
  dts: { generator: 'oxc' },
  outExtensions() { return { js: '.custom.js' }; },
  copy() { return ['assets']; },
}));
```
