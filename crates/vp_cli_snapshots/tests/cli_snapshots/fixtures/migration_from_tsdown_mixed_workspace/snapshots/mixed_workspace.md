# mixed_workspace

Install the pinned tsdown to build the root library. The other package already uses vp pack and still needs its own tsdown config merged during migration.

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
