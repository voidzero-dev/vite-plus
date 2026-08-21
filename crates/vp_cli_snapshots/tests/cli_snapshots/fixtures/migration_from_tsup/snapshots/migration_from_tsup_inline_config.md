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

Choose one of these manual migration methods:
  1. Run `vp dlx tsdown-migrate` in the project root.
  2. Use the tsdown migration skill:
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
