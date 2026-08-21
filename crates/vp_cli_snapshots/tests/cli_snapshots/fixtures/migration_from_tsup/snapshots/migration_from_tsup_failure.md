# migration_from_tsup_failure

## `vpt chmod +x tsdown-migrate-stub.mjs`

stub a tsdown-migrate failure

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a failed automatic migration should show manual options

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration failed.

Choose one of these manual migration methods:
  1. Run `vp dlx tsdown-migrate` in the project root.
  2. Use the tsdown migration skill:
     https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md

Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt stat-file tsup.config.ts --assert file`

the original tsup config is preserved

```
tsup.config.ts: file
```

## `vpt stat-file tsdown.config.ts --assert-not file`

no converted config is left behind

```
tsdown.config.ts: missing
```

## `vpt print-file package.json`

the tsup dependency and script are preserved

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
