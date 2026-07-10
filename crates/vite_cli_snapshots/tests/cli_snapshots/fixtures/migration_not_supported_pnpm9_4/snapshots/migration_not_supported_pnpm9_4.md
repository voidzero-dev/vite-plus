# migration_not_supported_pnpm9_4

## `vp migrate --no-interactive`

migration should fail because pnpm version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

✘ pnpm@9.4.0 is not supported by auto migration, please upgrade pnpm to >=9.5.0 first
Vite+ cannot automatically migrate this project yet.
```

## `vpt print-file package.json`

check package.json is not updated

```
{
  "devDependencies": {
    "vite": "^7.0.0"
  },
  "packageManager": "pnpm@9.4.0"
}
```
