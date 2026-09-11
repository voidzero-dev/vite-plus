# migration_from_tsup_config_collision

## `vpt write-file tsdown.config.ts 'export default { existing: true };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

an existing tsdown config should stop automatic migration before it overwrites files

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration was skipped because these tsdown configs already exist:
  tsdown.config.ts

Resolve this configuration conflict manually:
  1. Merge the tsup and tsdown configurations into `pack` in `vite.config.*`.
  2. Do not run `tsdown-migrate`. It can overwrite the existing tsdown configuration.

Use the tsdown migration skill for guidance:
  https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md
Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt print-file tsup.config.ts`

the original tsup config is unchanged

```
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  splitting: false,
});
```

## `vpt print-file tsdown.config.ts`

the existing tsdown config is unchanged

```
export default { existing: true };
```

## `vpt print-file package.json`

the tsup dependency and script are unchanged

```
{
  "name": "migration-from-tsup",
  "scripts": {
    "build": "tsup --config tsup.config.ts"
  },
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "^7.0.0"
  }
}
```
