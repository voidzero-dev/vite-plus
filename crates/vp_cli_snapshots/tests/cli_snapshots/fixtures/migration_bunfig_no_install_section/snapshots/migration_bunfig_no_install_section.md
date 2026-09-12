# migration_bunfig_no_install_section

## `vp migrate --no-interactive`

migration preserves bunfig.toml without an install section

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt print-file bunfig.toml`

check Bun configuration is unchanged

```
[run]
shell = "bun"
```
