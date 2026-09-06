# migration_upgrade_version_table_pnpm

## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.1.21","bundledVersions":{"vite":"8.0.0"}}'`

stub the installed vite-plus-core alias so the raw upstream vite version is read


## `vp migrate --no-interactive`

existing-Vite+ upgrade shows the toolchain version-change table with the raw vite row

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus            0.1.21 → <version>
    vite                 8.0.0  → <version>
    vitest               4.1.8  → <version>
    @vitest/coverage-v8  4.1.8  → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```
