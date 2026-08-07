# migration_not_supported_vitest3

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"3.2.4"}'`

stub the installed vitest so migrate reads the unsupported version offline


## `vp migrate --no-interactive`

migration should fail because vitest version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ vitest@3.2.4 in package.json is not supported by auto migration

Please upgrade vitest to version >=4.0.0 first
Vite+ cannot automatically migrate this project yet.
```

## `vpt print-file package.json`

check package.json is not updated

```
{
  "devDependencies": {
    "vitest": "<version>"
  },
  "packageManager": "pnpm@10.33.2"
}
```
