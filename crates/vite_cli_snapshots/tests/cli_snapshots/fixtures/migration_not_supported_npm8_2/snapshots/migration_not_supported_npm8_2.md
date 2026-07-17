# migration_not_supported_npm8_2

## `vp migrate --no-interactive`

migration should fail because npm version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ npm@8.2.0 is not supported by auto migration, please upgrade npm to >=8.3.0 first
Vite+ cannot automatically migrate this project yet.
```

## `vpt print-file package.json`

check package.json is not updated

```
{
  "devDependencies": {
    "vite": "^7.0.0"
  },
  "packageManager": "npm@8.2.0"
}
```
