# migration_oxc_config_conflict

## `vp migrate --no-interactive`

migration should refuse to start on conflicting Oxc configs

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ Conflicting Oxc configs:
  - the project root has `.oxlintrc.json` and `oxlint.config.ts` — oxlint allows only one config per directory.
Keep a single config per directory, then run `vp migrate` again.
```

## `vpt stat-file .oxlintrc.json --assert file`

both configs left untouched by the interrupt

```
.oxlintrc.json: file
```

## `vpt stat-file oxlint.config.ts --assert file`

```
oxlint.config.ts: file
```

## `vpt stat-file vite.config.ts --assert missing`

no file was written before the interrupt

```
vite.config.ts: missing
```

## `vpt print-file package.json`

package.json unchanged

```
{
  "name": "migration-oxc-config-conflict",
  "scripts": {
    "lint": "oxlint"
  },
  "devDependencies": {
    "oxlint": "^1.0.0",
    "vite": "^7.0.0"
  }
}
```
