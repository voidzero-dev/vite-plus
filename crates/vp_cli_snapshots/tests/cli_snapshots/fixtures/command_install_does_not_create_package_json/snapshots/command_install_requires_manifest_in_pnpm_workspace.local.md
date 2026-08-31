# command_install_requires_manifest_in_pnpm_workspace

A workspace marker alone does not satisfy the package.json requirement.

## `vp install --silent`

install should reject a workspace without package.json

**Exit code:** 1

```
error: Package not found in workspace: `<workspace>/pnpm-workspace`
```

## `vpt stat-file package.json --assert-not file`

install should not create package.json

```
package.json: missing
```
