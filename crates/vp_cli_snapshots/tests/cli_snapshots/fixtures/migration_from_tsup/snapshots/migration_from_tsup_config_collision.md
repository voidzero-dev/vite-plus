# migration_from_tsup_config_collision

## `vpt write-file tsdown.config.ts 'export default { existing: true };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

an existing tsdown config should stop automatic migration before it overwrites files

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration was skipped because these tsdown config files already exist:
  tsdown.config.ts

Choose one of these manual migration methods:
  1. Run `vp dlx tsdown-migrate` in the project root.
  2. Use the tsdown migration skill:
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
