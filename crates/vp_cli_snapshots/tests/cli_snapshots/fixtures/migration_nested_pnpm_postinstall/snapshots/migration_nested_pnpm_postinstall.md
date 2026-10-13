# migration_nested_pnpm_postinstall

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

migrate a standalone project whose postinstall installs a nested pnpm project

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vp install`

install completes without postinstall recursion


## `vpt print-file web/pnpm-workspace.yaml`

the child keeps its install boundary

```
packages:
  - "."
```

## `vpt stat-file web/node_modules/test-vite-plus-install --assert-not missing`

the child's dependency was installed

```
web/node_modules/test-vite-plus-install: symlink
```
