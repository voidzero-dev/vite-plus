# migration_nested_pnpm11_overrides_failure

## `vpt json-edit package.json packageManager pnpm@11.25.0`


## `vpt json-edit web/package.json packageManager pnpm@11.25.0`


## `vpt json-edit web/package.json pnpm.overrides '{"test-vite-plus-install":"file:./missing"}'`


## `vpt json-edit web/package.json scripts.postinstall 'echo yes > postinstall-ran'`


## `cd web && pnpm install --frozen-lockfile --lockfile-only --ignore-scripts`

the original lockfile is valid before the ignored override becomes active


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a failed child lockfile update reports a warning and completes the root install and migration summary

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
! Warnings:
  - Could not update dependencies for the nested project at <workspace>/web.
✓ Lockfile passes supply-chain policies (verified <duration> ago)
[ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND] Could not install from "<workspace>/web/missing" as it does not exist.

This error happened while installing a direct dependency of <workspace>/web

[WARN] The "pnpm" field in package.json is no longer read by pnpm. The following keys were ignored: "pnpm.overrides". See https://pnpm.io/settings for the new home of each setting.
Run `vp install --lockfile-only --ignore-scripts --no-frozen-lockfile` in that directory.
```

## `vpt stat-file node_modules/vite-plus --assert-not missing`

the root's final install completed despite the child lockfile update failure

```
node_modules/vite-plus: symlink
```

## `vpt print-file web/pnpm-workspace.yaml`

the child's install boundary is preserved

```
packages:
  - "."
overrides:
  test-vite-plus-install: file:./missing
```

## `vpt replace-file-content web/pnpm-workspace.yaml file:./missing 1.0.0`

repair the child's effective override


## `cd web && vp install --lockfile-only --ignore-scripts --no-frozen-lockfile`

the reported manual recovery command updates the child's lockfile


## `cd web && pnpm install --frozen-lockfile --ignore-scripts`

the child's frozen install succeeds after manual recovery


## `vpt stat-file web/postinstall-ran --assert missing`

neither update attempt ran the child's lifecycle scripts

```
web/postinstall-ran: missing
```
