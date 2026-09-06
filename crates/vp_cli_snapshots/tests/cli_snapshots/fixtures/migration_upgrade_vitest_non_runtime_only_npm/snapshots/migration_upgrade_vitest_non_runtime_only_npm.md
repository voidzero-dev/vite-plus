# migration_upgrade_vitest_non_runtime_only_npm

## `vp migrate --no-interactive`

non-runtime @vitest packages must not keep a vitest pin

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

package.json
  1:1 REVIEW [legacy-dependency] Review the direct @vitest/ws-client dependency after migrating its imports; it is no longer part of the bundled Vitest graph.
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus      latest → <version>
    vite                  → <version>
    @vitest/utils  4.1.8  → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 2 review items

package.json
  1:1 REVIEW [legacy-dependency] Review the direct @vitest/ws-client dependency after migrating its imports; it is no longer part of the bundled Vitest graph.
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

internal packages align, eslint plugin stays independent, vitest is removed

```
{
  "name": "migration-upgrade-vitest-non-runtime-only-npm",
  "devDependencies": {
    "@vitest/eslint-plugin": "^1.6.0",
    "@vitest/utils": "<version>",
    "@vitest/ws-client": "^4.1.8",
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
