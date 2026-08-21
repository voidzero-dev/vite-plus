# migration_from_tsup_inline_config

## `vpt rm tsup.config.ts`


## `vpt json-edit package.json scripts.build tsup`


## `vpt json-edit package.json tsup '{"entry":["src/index.ts"]}'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

an inline tsup config should stop automatic migration

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration was skipped because these inline tsup configs cannot be migrated automatically:
  package.json#tsup

Resolve this inline configuration manually:
  1. Move each `package.json#tsup` configuration into `pack` in `vite.config.*`.
  2. Do not run `tsdown-migrate`. Vite+ Pack does not read `package.json#tsdown`.

Use the tsdown migration skill for guidance:
  https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md
Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt print-file package.json`

the inline tsup config is unchanged

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
  "tsup": {
    "entry": [
      "src/index.ts"
    ]
  }
}
```
