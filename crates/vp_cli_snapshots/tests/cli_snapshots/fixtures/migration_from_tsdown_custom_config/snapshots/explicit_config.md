# explicit_config

Build with the published, pinned tsdown so the test checks the output of the selected config after a real dependency install.

## `vp install --ignore-scripts`


## `vp run bundle`


## `vpt print-file lib-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```

## `vpt rm -r lib-out`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - package.json script "bundle" selects a tsdown config. Kept tsdown in this workspace. Move the selected config into `pack` in `vite.config.ts` before switching to `vp pack` without the config flag. See https://viteplus.dev/guide/migrate#tsdown
```

## `vpt print-file package.json`

```
{
  "name": "migration-from-tsdown-custom-config",
  "private": true,
  "type": "module",
  "scripts": {
    "bundle": "tsdown --config tsdown.custom.config.ts"
  },
  "devDependencies": {
    "tsdown": "catalog:build",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.11.2"
}
```

## `vpt print-file tsdown.custom.config.ts`

```
import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/library.ts"],
  outDir: "lib-out",
  dts: false,
});
```

## `vp run bundle`


## `vpt print-file lib-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```

## `vpt stat-file dist/tsdown.custom.config.mjs --assert missing`

```
dist/tsdown.custom.config.mjs: missing
```

## `vp migrate --no-interactive`


## `vpt rm -r lib-out`


## `vp run bundle`


## `vpt print-file lib-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```

## `vp pack src/library.ts --out-dir positional-out`


## `vpt print-file positional-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```
