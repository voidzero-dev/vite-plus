# migration_upgrade_version_table_pnpm

## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.1.21","bundledVersions":{"vite":"8.0.0"}}'`

stub the installed vite-plus-core alias so the raw upstream vite version is read


## `vp migrate --no-interactive`

existing-Vite+ upgrade shows the toolchain version-change table with the raw vite row

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus            0.1.21 → <version>
    vite                 8.0.0  → <version>
    vitest               3.2.4  → <version>
    @vitest/coverage-v8  3.2.4  → <version>
• Package manager settings configured
```
