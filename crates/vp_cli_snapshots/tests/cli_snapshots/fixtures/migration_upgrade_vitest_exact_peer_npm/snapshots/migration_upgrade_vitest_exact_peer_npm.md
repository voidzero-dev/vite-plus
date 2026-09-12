# migration_upgrade_vitest_exact_peer_npm

## `vp migrate --no-interactive`

exact @vitest peers require a package-local vitest

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus            latest → <version>
    vite                        → <version>
    vitest               latest → <version>
    @vitest/coverage-v8  4.1.8  → <version>
    @vitest/ui           4.1.8  → <version>
    @vitest/utils        4.1.8  → <version>
    @vitest/web-worker   4.1.8  → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

ecosystem packages and vitest should align to the bundled version

```
{
  "name": "migration-upgrade-vitest-exact-peer-npm",
  "devDependencies": {
    "@vitest/coverage-v8": "<version>",
    "@vitest/eslint-plugin": "^1.6.0",
    "@vitest/ui": "<version>",
    "@vitest/utils": "<version>",
    "@vitest/web-worker": "<version>",
    "vite-plus": "<version>",
    "vitest": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
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
