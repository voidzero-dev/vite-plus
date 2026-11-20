# dynamic_config

The root script selects a config through an environment variable. Build both packages, then select the child's original JSON from the same script after a second migration.

## `vpt json-edit package.json scripts.bundle 'export TSDOWN_CONFIG=tsdown.custom.config.ts && tsdown --config "$TSDOWN_CONFIG"'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
• Package manager settings configured
! Warnings:
  - package.json script "bundle" selects a tsdown config. Kept tsdown in this workspace. Move the selected config into `pack` in `vite.config.ts` before switching to `vp pack` without the config flag. See https://viteplus.dev/guide/migrate#tsdown
```

## `vp run bundle`


## `vpt print-file custom-out/root-library.mjs`

```
// src/root-library.ts
const rootAnswer = 42;
export { rootAnswer };
```

## `cd packages/legacy-pack && vp run build`


## `vpt stat-file packages/legacy-pack/lib-out/library.mjs --assert file`

```
packages/legacy-pack/lib-out/library.mjs: file
```

## `vpt print-file packages/legacy-pack/lib-out/library.mjs`

```
// src/library.ts
const libraryAnswer = 17;
export { libraryAnswer };
```

## `vpt stat-file packages/legacy-pack/dist/index.mjs --assert missing`

```
packages/legacy-pack/dist/index.mjs: missing
```

## `vpt rm -r packages/legacy-pack/lib-out`


## `vpt json-edit package.json scripts.bundle 'export TSDOWN_CONFIG=packages/legacy-pack/tsdown.config.json && tsdown --config "$TSDOWN_CONFIG"'`


## `vp migrate --no-interactive`


## `vpt stat-file packages/legacy-pack/tsdown.config.json --assert file`

```
packages/legacy-pack/tsdown.config.json: file
```

## `vp run bundle`


## `vpt print-file packages/legacy-pack/lib-out/library.mjs`

```
// src/library.ts
const libraryAnswer = 17;
export { libraryAnswer };
```
