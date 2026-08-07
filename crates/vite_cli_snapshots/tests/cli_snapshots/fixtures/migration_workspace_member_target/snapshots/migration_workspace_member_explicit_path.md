# migration_workspace_member_explicit_path

## `vp migrate . --no-interactive --no-agent --no-editor --no-hooks`

reject an explicit workspace-member target before changing files

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Cannot migrate a workspace member independently. Run `vp migrate` from the workspace root at <workspace>.
```

## `vpt print-file ../../package.json`

workspace root remains unchanged

```
{
  "name": "workspace-root",
  "private": true,
  "workspaces": [
    "vendor/*"
  ]
}
```

## `vpt print-file package.json`

workspace member remains unchanged

```
{
  "name": "workspace-member",
  "private": true,
  "devDependencies": {
    "vitest": "<version>"
  }
}
```

## `vpt stat-file ../../pnpm-workspace.yaml --assert missing`

no package-manager files are created at the workspace root

```
../../pnpm-workspace.yaml: missing
```
