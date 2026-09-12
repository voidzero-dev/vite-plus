# migration_bunfig_missing

## `vp migrate --no-interactive`

migration does not create bunfig.toml

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt stat-file bunfig.toml --assert-not file`

check Bun configuration remains absent

```
bunfig.toml: missing
```
