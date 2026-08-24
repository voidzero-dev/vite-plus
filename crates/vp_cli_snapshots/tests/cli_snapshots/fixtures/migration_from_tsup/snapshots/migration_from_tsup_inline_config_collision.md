# migration_from_tsup_inline_config_collision

## `vpt rm tsup.config.ts`


## `vpt json-edit package.json scripts.build tsup`


## `vpt json-edit package.json tsup '{"entry":["src/index.ts"]}'`


## `vpt json-edit package.json tsdown '{"entry":["src/existing.ts"]}'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

an existing inline tsdown config should stop automatic migration

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration was skipped because these tsdown configs already exist:
  package.json#tsdown

Resolve this configuration conflict manually:
  1. Merge the tsup and tsdown configurations into `pack` in `vite.config.*`.
  2. Do not run `tsdown-migrate`. It can overwrite the existing tsdown configuration.

Use the tsdown migration skill for guidance:
  https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md
Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt print-file package.json`

both inline configs are unchanged

```
{
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "^7.0.0"
  },
  "name": "migration-from-tsup",
  "scripts": {
    "build": "tsup"
  },
  "tsdown": {
    "entry": [
      "src/existing.ts"
    ]
  },
  "tsup": {
    "entry": [
      "src/index.ts"
    ]
  }
}
```
