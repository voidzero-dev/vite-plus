# migration_not_supported_vite6

## `vpt write-file node_modules/vite/package.json '{"name":"vite","version":"6.4.3"}'`

stub the installed vite so migrate reads the unsupported version offline


## `vp migrate --no-interactive`

migration should fail because vite version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ vite@6.4.3 in package.json is not supported by auto migration

Please upgrade vite to version >=7.0.0 first
Vite+ cannot automatically migrate this project yet.
```

## `vpt print-file package.json`

check package.json is not updated

```
{
  "devDependencies": {
    "vite": "^6.0.0"
  },
  "packageManager": "pnpm@10.33.2"
}
```
