# pnpm_uses_powershell_for_interactive_shim

## `node prepare.cjs pnpm`


## `pnpm --flag 'value with spaces' path\file`

The pnpm.exe proxy uses the sibling PowerShell launcher to avoid a batch termination prompt on Ctrl+C.

```
launcher=ps1
arguments, environment, cwd, and Node selection preserved
```

## `pnpm --flag 'value with spaces' path\file`

Piped stdin keeps the batch launcher, since package-manager PowerShell wrappers may read from stdin.

```
launcher=cmd
arguments, environment, cwd, and Node selection preserved
```

## `SHIM_TEST_EXIT_CODE=7 pnpm --flag 'value with spaces' path\file`

The PowerShell launcher preserves the child's exit code.

**Exit code:** 7

```
launcher=ps1
arguments, environment, cwd, and Node selection preserved
```

## `vpt rm $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm.ps1`


## `pnpm --flag 'value with spaces' path\file`

A managed launcher without a sibling .ps1 still uses .cmd.

```
launcher=cmd
arguments, environment, cwd, and Node selection preserved
```
