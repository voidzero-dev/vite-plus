# json_failure

## `vpt write-file .npmrc 'json=true
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

The migration keeps pnpm's JSON error object and exit code.

**Exit code:** 7

```
VITE+ - The Unified Toolchain for the Web

> pnpm9-lifecycle-config@ postinstall <workspace>
> node postinstall.cjs

Configuring the project
{
  "error": {
    "code": "ELIFECYCLE",
    "message": "pnpm9-lifecycle-config@ postinstall: `node postinstall.cjs`\nExit status 7"
  }
}
Fixture postinstall failed
● You may need to run "vp install" manually in <workspace>
{
  "error": {
    "code": "ELIFECYCLE",
    "message": "pnpm9-lifecycle-config@ postinstall: `node postinstall.cjs`\nExit status 7"
  }
}
● You may need to run "vp install" manually in <workspace>
warn: Dependency installation failed (exit code 7). Run `vp install` manually in <workspace> to resync node_modules.
warn: Dependency installation failed (exit code 7). Run `vp install` manually in <workspace> to resync node_modules.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
! Warnings:
  - Dependency installation failed (exit code 7). Run `vp install` manually in <workspace> to resync node_modules.
  - Dependency installation failed (exit code 7). Run `vp install` manually in <workspace> to resync node_modules.
```
