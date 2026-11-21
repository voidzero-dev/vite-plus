# silent_failure

## `vpt write-file .npmrc 'reporter=silent
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

The migration keeps the postinstall error and exit code without displaying install progress.

**Exit code:** 7

```
VITE+ - The Unified Toolchain for the Web

● > pnpm9-lifecycle-config@ postinstall <workspace>
  > node postinstall.cjs

  Configuring the project
■ Fixture postinstall failed

● You may need to run "vp install" manually in <workspace>
●
■
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
