# migration_from_tsup_custom_config

## `vpt mkdir -p configs`


## `vpt write-file configs/legacy.ts 'export default {};
'`


## `vpt json-edit package.json scripts.build 'tsup --config configs/legacy.ts'`


## `vpt json-edit package.json scripts.irregular 'tsup --config ././tsup.config.ts'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

unsupported config paths should stop automatic migration before any files change

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration was skipped because these scripts use configs that cannot be migrated automatically:
  package.json#build -> configs/legacy.ts
  package.json#irregular -> ././tsup.config.ts

Choose one of these manual migration methods:
  1. Run `vp dlx tsdown-migrate` in the project root.
  2. Use the tsdown migration skill:
     https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md
Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt print-file tsup.config.ts`

the standard config is unchanged

```
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  splitting: false,
});
```

## `vpt print-file configs/legacy.ts`

the custom config is unchanged

```
export default {};
```

## `vpt print-file package.json`

the unsupported scripts and tsup dependency are unchanged

```
{
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "^7.0.0"
  },
  "name": "migration-from-tsup",
  "scripts": {
    "build": "tsup --config configs/legacy.ts",
    "irregular": "tsup --config ././tsup.config.ts"
  }
}
```
