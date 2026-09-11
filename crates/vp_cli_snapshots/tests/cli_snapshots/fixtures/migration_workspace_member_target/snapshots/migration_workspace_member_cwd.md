# migration_workspace_member_cwd

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

vp migrate rejects a workspace member before it changes files

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vite+ cannot migrate a workspace member. Run `vp migrate` from the workspace root at <workspace>.
```

## `vpt print-file ../../package.json`

the workspace root package.json is unchanged

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

the workspace member package.json is unchanged

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

the migration created no package-manager files at the workspace root

```
../../pnpm-workspace.yaml: missing
```
