# migration_nested_pnpm_workspace_member

## `vpt mkdir vendor/packages/app/examples`


## `vpt cp -r web vendor/packages/app/examples/demo`


## `vpt rm -r web`


## `vpt json-edit package.json scripts.postinstall 'cd vendor/packages/app/examples/demo && pnpm install'`


## `cd vendor/packages/app/examples/demo && pnpm install --frozen-lockfile`

install the independent demo before migration


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

preserve the independent demo below a workspace member

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


## `cd vendor/packages/app/examples/demo && pnpm install --frozen-lockfile`

the demo's frozen install still succeeds


## `vpt print-file vendor/packages/app/examples/demo/pnpm-workspace.yaml`

the demo keeps its install boundary

```
packages:
  - "."
```

## `vpt stat-file vendor/packages/app/pnpm-workspace.yaml --assert missing`

the workspace member gets no boundary

```
vendor/packages/app/pnpm-workspace.yaml: missing
```

## `vpt stat-file vendor/packages/app/examples/demo/node_modules/test-vite-plus-install --assert-not missing`

the demo's dependency was installed

```
vendor/packages/app/examples/demo/node_modules/test-vite-plus-install: symlink
```
