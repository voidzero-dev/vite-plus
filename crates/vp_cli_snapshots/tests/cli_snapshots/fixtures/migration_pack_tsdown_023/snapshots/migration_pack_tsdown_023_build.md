# migration_pack_tsdown_023_build

Build a migrated library and check that static assets and declarations are emitted.

## `vpt write-file vite.config.ts 'export default { pack: { entry: '\''src/index.ts'\'', bundle: false, publicDir: '\''public'\'', removeNodeProtocol: true, dts: { oxc: true, cjsReexport: false } } };'`


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

## `vp run build`

```
VITE+ - The Unified Toolchain for the Web

$ vp pack --copy public ⊘ cache disabled
ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt stat-file dist/index.mjs --assert file`

```
dist/index.mjs: file
```

## `vpt stat-file dist/index.d.mts --assert file`

```
dist/index.d.mts: file
```

## `vpt print-file dist/public/asset.txt`

```
copied asset
```
