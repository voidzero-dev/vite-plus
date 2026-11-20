# active_pack_override

Keep the standalone build when the active pack overrides the converted config's entry.

## `vpt json-edit package.json scripts.build tsdown`


## `vpt write-file tsdown.config.ts 'import { defineConfig } from '\''tsdown'\''; export default defineConfig({ entry: ['\''src/library.ts'\''] });'`


## `vpt write-file src/browser.ts 'export const answer = 21;'`


## `vp install --ignore-scripts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file vite-pack.config.ts --assert file`

```
vite-pack.config.ts: file
```

## `vpt json-edit package.json scripts '{"build":"tsdown"}'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import config from '\''./vite-pack.config.js'\''; export default defineConfig({ pack: { ...config, entry: ['\''src/browser.ts'\''] } });'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

! Warnings:
  - Kept standalone tsdown because migration could not verify that vite.config.ts matches tsdown.config.ts. Please manually merge the configs before switching to `vp pack`. See https://viteplus.dev/guide/migrate#tsdown
This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "tsdown": "catalog:build",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsdown-custom-config",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "scripts": {
    "build": "tsdown"
  },
  "type": "module"
}
```

## `vp run build`


## `vpt stat-file dist/library.mjs --assert file`

```
dist/library.mjs: file
```

## `vpt stat-file dist/browser.mjs --assert missing`

```
dist/browser.mjs: missing
```
