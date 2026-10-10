# migration_workspace_members

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

migrate a workspace without a lockfile or package-manager declaration

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file pnpm-workspace.yaml`

the selected pnpm manager retains the workspace members

```
packages:
  - packages/*
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite@*: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `vp run -r build`

both migrated workspace members remain runnable


## `vpt stat-file packages/a/dist/index.html --assert file`

```
packages/a/dist/index.html: file
```

## `vpt stat-file packages/b/dist/index.html --assert file`

```
packages/b/dist/index.html: file
```
