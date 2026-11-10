# migration_from_tsdown_023

Migrate standalone tsdown 0.23.0 without adding legacy dependency or ATTW defaults, and preserve the result on a second migration.

## `vpt json-edit package.json devDependencies.tsdown 0.23.0`


## `vpt json-edit package.json scripts.build 'tsdown --copy public'`


## `vpt write-file node_modules/tsdown/package.json '{"name":"tsdown","version":"0.23.0"}'`


## `vpt cp modern.config.txt tsdown.config.ts`


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

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
  outDir: 'dist',
  format: ['esm', 'cjs'],
  dts: true,
  unbundle: true,
  copy: 'public',
  deps: { onlyBundle: false },
  attw: true,
});
```

## `vpt print-file vite.config.ts`

```
import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  pack: tsdownConfig,
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsdown",
  "scripts": {
    "build": "vp pack --copy public",
    "build:dts": "vp pack --dts",
    "build:watch": "vp pack --watch",
    "prepare": "vp config"
  }
}
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
  outDir: 'dist',
  format: ['esm', 'cjs'],
  dts: true,
  unbundle: true,
  copy: 'public',
  deps: { onlyBundle: false },
  attw: true,
});
```

## `vpt print-file vite.config.ts`

```
import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  pack: tsdownConfig,
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsdown",
  "scripts": {
    "build": "vp pack --copy public",
    "build:dts": "vp pack --dts",
    "build:watch": "vp pack --watch",
    "prepare": "vp config"
  }
}
```
