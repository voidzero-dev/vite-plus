# migration_upgrade_bun_catalog

## `vp migrate --no-interactive`

existing bun Vite+ catalog WORKSPACE gains a direct vite edge for bun#8406 (bun resolves catalog: only inside a workspace)

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  bun <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

the direct vite edge is catalog: (resolving through the bun catalog), not a concrete alias

```
{
  "name": "migration-upgrade-bun-catalog",
  "workspaces": [
    "packages/*"
  ],
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "overrides": {
    "vite": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "catalog": {
    "vite-plus": "<version>",
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```
