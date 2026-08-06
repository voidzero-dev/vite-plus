# migration_upgrade_vitest_non_runtime_only_npm

## `vp migrate --no-interactive`

non-runtime @vitest packages must not keep a vitest pin

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus          latest → <version>
    vite                      → <version>
    @vitest/utils      4.1.8  → <version>
    @vitest/ws-client  4.1.8  → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

internal packages align, eslint plugin stays independent, vitest is removed

```
{
  "name": "migration-upgrade-vitest-non-runtime-only-npm",
  "devDependencies": {
    "@vitest/eslint-plugin": "^1.6.0",
    "@vitest/utils": "<version>",
    "@vitest/ws-client": "<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```
