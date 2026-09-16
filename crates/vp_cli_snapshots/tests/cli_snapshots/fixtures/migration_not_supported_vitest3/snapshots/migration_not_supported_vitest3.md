# migration_not_supported_vitest3

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"3.2.4"}'`

stub the installed vitest so migrate reads the unsupported version offline


## `vp migrate --no-interactive`

migration should fail because vitest version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

package.json
  1:1 BLOCK [source-version] Upgrade the original project to Vitest 4 before running this migration.
    Docs: https://viteplus.dev/guide/vitest-v5#before-you-migrate
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
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
