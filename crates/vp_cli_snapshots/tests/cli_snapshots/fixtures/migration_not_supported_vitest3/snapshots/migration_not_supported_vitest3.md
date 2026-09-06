# migration_not_supported_vitest3

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"3.2.4"}'`

stub the installed vitest so migrate reads the unsupported version offline


## `vp migrate --no-interactive`

migration should fail because vitest version is not supported

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items (1 block dependency updates)

package.json
  1:1 BLOCK [source-version] Upgrade the original project to Vitest 4 before running this migration.
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
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
