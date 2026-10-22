# migration_nested_pnpm11_overrides

## `vpt json-edit package.json packageManager pnpm@11.25.0`


## `vpt json-edit web/package.json packageManager pnpm@11.25.0`


## `vpt json-edit web/package.json pnpm.overrides '{"test-vite-plus-install":"1.0.0"}'`


## `vpt json-edit web/package.json scripts.postinstall 'echo yes > postinstall-ran'`


## `cd web && pnpm install --frozen-lockfile --lockfile-only --ignore-scripts`

the child's original lockfile is valid while pnpm 11 ignores package.json overrides


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

copying the child's overrides to YAML updates its lockfile

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt stat-file web/postinstall-ran --assert missing`

updating the child's lockfile does not run its lifecycle scripts

```
web/postinstall-ran: missing
```

## `cd web && pnpm install --frozen-lockfile --ignore-scripts`

the child's frozen install succeeds immediately after migration


## `vpt print-file web/pnpm-workspace.yaml`

the child keeps its install boundary and migrated overrides

```
packages:
  - "."
overrides:
  test-vite-plus-install: 1.0.0
```
