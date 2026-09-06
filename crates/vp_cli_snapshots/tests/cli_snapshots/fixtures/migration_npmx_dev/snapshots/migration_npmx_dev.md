# migration_npmx_dev

## `vp migrate --no-interactive`

npmx.dev shape: existing Vite+ upgrade, concrete @vitest/* should move into the catalog

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default and uses exact browser locators. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus                   latest → <version>
    vite                               → <version>
    vitest                      4.1.10 → <version>
    @vitest/browser-playwright  4.1.10 → <version>
    @vitest/coverage-v8         4.1.10 → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default and uses exact browser locators. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

@vitest/browser-playwright and @vitest/coverage-v8 should become catalog:

```
{
  "name": "npmx",
  "private": true,
  "devDependencies": {
    "@vitest/browser-playwright": "catalog:",
    "@vitest/coverage-v8": "catalog:",
    "playwright": "1.60.0",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

the default catalog should own the aligned @vitest/* packages

```
packages:
  - .
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/browser-playwright': <version>
  '@vitest/coverage-v8': <version>
overrides:
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```
