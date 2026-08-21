# migration_from_tsup_monorepo_failure

## `vpt chmod +x tsdown-migrate-failure-stub.mjs`

stub a failure after package a succeeds

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a later package failure should roll back every package

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

Automatic tsup migration failed.

Choose one of these manual migration methods:
  1. Run `vp dlx tsdown-migrate` in packages/b.
  2. Use the tsdown migration skill:
     https://github.com/rolldown/tsdown/blob/main/skills/tsdown-migrate/SKILL.md

Complete the tsup migration manually, then re-run `vp migrate`.
```

## `vpt stat-file packages/a/tsup.config.ts --assert file`

package a original config is restored

```
packages/a/tsup.config.ts: file
```

## `vpt stat-file packages/a/tsdown.config.ts --assert-not file`

package a converted config is removed

```
packages/a/tsdown.config.ts: missing
```

## `vpt print-file packages/a/package.json`

package a manifest is restored

```
{
  "name": "a",
  "type": "module",
  "scripts": {
    "build": "tsup --config tsup.config.ts"
  },
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "^7.0.0"
  }
}
```

## `vpt stat-file packages/b/tsup.config.ts --assert file`

package b original config is restored

```
packages/b/tsup.config.ts: file
```

## `vpt stat-file packages/b/tsdown.config.ts --assert-not file`

package b partial config is removed

```
packages/b/tsdown.config.ts: missing
```

## `vpt print-file packages/b/package.json`

package b manifest is restored

```
{
  "name": "b",
  "type": "module",
  "scripts": {
    "build": "tsup"
  },
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "^7.0.0"
  }
}
```
